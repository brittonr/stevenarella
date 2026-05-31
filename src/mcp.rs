// Copyright 2026
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your option.

use serde_json::{json, Value};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub const DEFAULT_MCP_TOKEN_ENV: &str = "STEVENARELLA_MCP_TOKEN";

const REASON_EMPTY_TOKEN_ENV_NAME: &str = "empty_token_env_name";
const REASON_EMPTY_TOKEN_VALUE: &str = "empty_token_value";
const TCP_ACCEPT_IDLE_SLEEP_MILLIS: u64 = 10;
const JSONRPC_PARSE_ERROR: i64 = -32700;
const JSONRPC_INVALID_REQUEST: i64 = -32600;
const JSONRPC_METHOD_NOT_FOUND: i64 = -32601;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpTransportOptions {
    pub stdio: bool,
    pub listen: Option<String>,
    pub token_env: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedMcpTransport {
    pub endpoints: Vec<McpEndpoint>,
    pub stdout_must_remain_clean: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpEndpoint {
    Stdio,
    Tcp {
        bind_addr: SocketAddr,
        auth: TcpAuth,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TcpAuth {
    NotRequiredForLoopback,
    TokenEnv { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartedMcpEndpoint {
    Stdio,
    Tcp { local_addr: SocketAddr },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpTransportError {
    MalformedListenAddress(String),
    MissingTokenEnvForNonLoopback {
        bind_addr: SocketAddr,
    },
    InvalidTokenEnvName {
        reason: &'static str,
    },
    MissingTokenValue {
        token_env: String,
    },
    InvalidTokenValue {
        token_env: String,
        reason: &'static str,
    },
}

#[derive(Debug)]
pub enum McpTransportStartError {
    Io(io::Error),
}

pub struct McpTransportRuntime {
    pub endpoints: Vec<StartedMcpEndpoint>,
    pub stdout_must_remain_clean: bool,
    shutdown_flags: Vec<Arc<AtomicBool>>,
    join_handles: Vec<JoinHandle<()>>,
}

impl Drop for McpTransportRuntime {
    fn drop(&mut self) {
        for shutdown_flag in &self.shutdown_flags {
            shutdown_flag.store(true, Ordering::Release);
        }
    }
}

impl From<io::Error> for McpTransportStartError {
    fn from(err: io::Error) -> Self {
        McpTransportStartError::Io(err)
    }
}

impl McpTransportOptions {
    pub fn from_cli(stdio: bool, listen: Option<String>, token_env: Option<String>) -> Self {
        McpTransportOptions {
            stdio,
            listen,
            token_env,
        }
    }

    pub fn has_transport(&self) -> bool {
        self.stdio || self.listen.is_some()
    }
}

impl McpTransportRuntime {
    pub fn join_handle_count(&self) -> usize {
        self.join_handles.len()
    }
}

pub fn validate_process_transport_options(
    options: &McpTransportOptions,
) -> Result<ValidatedMcpTransport, McpTransportError> {
    validate_transport_options(options, |name| std::env::var(name).ok())
}

pub fn validate_transport_options<F>(
    options: &McpTransportOptions,
    token_lookup: F,
) -> Result<ValidatedMcpTransport, McpTransportError>
where
    F: Fn(&str) -> Option<String>,
{
    let mut endpoints = Vec::new();
    let mut stdout_must_remain_clean = false;

    if options.stdio {
        endpoints.push(McpEndpoint::Stdio);
        stdout_must_remain_clean = true;
    }

    if let Some(listen) = options.listen.as_deref() {
        endpoints.push(validate_tcp_endpoint(
            listen,
            options.token_env.as_deref(),
            &token_lookup,
        )?);
    }

    Ok(ValidatedMcpTransport {
        endpoints,
        stdout_must_remain_clean,
    })
}

pub fn start_process_transport(
    validated: ValidatedMcpTransport,
) -> Result<McpTransportRuntime, McpTransportStartError> {
    start_transport_with_stdio(validated, io::stdin(), io::stdout())
}

pub fn start_transport_runtime(
    validated: ValidatedMcpTransport,
) -> Result<McpTransportRuntime, McpTransportStartError> {
    start_transport_runtime_inner(validated, None::<(io::Empty, io::Sink)>)
}

pub fn start_transport_with_stdio<R, W>(
    validated: ValidatedMcpTransport,
    reader: R,
    writer: W,
) -> Result<McpTransportRuntime, McpTransportStartError>
where
    R: io::Read + Send + 'static,
    W: io::Write + Send + 'static,
{
    start_transport_runtime_inner(validated, Some((reader, writer)))
}

pub fn run_jsonrpc_lines<R, W>(mut reader: R, mut writer: W) -> io::Result<()>
where
    R: BufRead,
    W: Write,
{
    let mut line = String::new();
    loop {
        line.clear();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            return Ok(());
        }
        if let Some(response) = handle_jsonrpc_line(line.trim_end_matches(['\r', '\n'])) {
            writeln!(writer, "{}", response)?;
            writer.flush()?;
        }
    }
}

pub fn handle_jsonrpc_line(line: &str) -> Option<String> {
    let value = match serde_json::from_str::<Value>(line) {
        Ok(value) => value,
        Err(_) => {
            return Some(jsonrpc_error(
                Value::Null,
                JSONRPC_PARSE_ERROR,
                "parse error",
            ))
        }
    };

    let Some(object) = value.as_object() else {
        return Some(jsonrpc_error(
            Value::Null,
            JSONRPC_INVALID_REQUEST,
            "request must be an object",
        ));
    };

    let id = object.get("id").cloned();
    let method = object.get("method").and_then(Value::as_str);
    let Some(method) = method else {
        return id.map(|id| jsonrpc_error(id, JSONRPC_INVALID_REQUEST, "missing method"));
    };

    match method {
        "initialize" => id.map(|id| {
            jsonrpc_result(
                id,
                json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {
                        "name": "stevenarella",
                        "version": env!("CARGO_PKG_VERSION"),
                    },
                    "capabilities": {
                        "tools": {},
                        "resources": {},
                    },
                }),
            )
        }),
        "tools/list" => id.map(|id| jsonrpc_result(id, json!({ "tools": [] }))),
        "resources/list" => id.map(|id| jsonrpc_result(id, json!({ "resources": [] }))),
        "ping" => id.map(|id| jsonrpc_result(id, json!({}))),
        method if method.starts_with("notifications/") => None,
        _ => id.map(|id| jsonrpc_error(id, JSONRPC_METHOD_NOT_FOUND, "method not found")),
    }
}

fn start_transport_runtime_inner<R, W>(
    validated: ValidatedMcpTransport,
    stdio: Option<(R, W)>,
) -> Result<McpTransportRuntime, McpTransportStartError>
where
    R: io::Read + Send + 'static,
    W: io::Write + Send + 'static,
{
    let mut endpoints = Vec::with_capacity(validated.endpoints.len());
    let mut shutdown_flags = Vec::new();
    let mut join_handles = Vec::new();
    let mut stdio = stdio;

    for endpoint in validated.endpoints {
        match endpoint {
            McpEndpoint::Stdio => {
                endpoints.push(StartedMcpEndpoint::Stdio);
                if let Some((reader, writer)) = stdio.take() {
                    join_handles.push(thread::spawn(move || {
                        let reader = BufReader::new(reader);
                        let writer = BufWriter::new(writer);
                        let _ = run_jsonrpc_lines(reader, writer);
                    }));
                }
            }
            McpEndpoint::Tcp { bind_addr, .. } => {
                let listener = TcpListener::bind(bind_addr)?;
                listener.set_nonblocking(true)?;
                let local_addr = listener.local_addr()?;
                let shutdown_flag = Arc::new(AtomicBool::new(false));
                let thread_shutdown_flag = Arc::clone(&shutdown_flag);
                join_handles.push(thread::spawn(move || {
                    accept_tcp_jsonrpc(listener, thread_shutdown_flag);
                }));
                shutdown_flags.push(shutdown_flag);
                endpoints.push(StartedMcpEndpoint::Tcp { local_addr });
            }
        }
    }

    Ok(McpTransportRuntime {
        endpoints,
        stdout_must_remain_clean: validated.stdout_must_remain_clean,
        shutdown_flags,
        join_handles,
    })
}

fn accept_tcp_jsonrpc(listener: TcpListener, shutdown_flag: Arc<AtomicBool>) {
    while !shutdown_flag.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((stream, _)) => {
                thread::spawn(move || {
                    let _ = serve_tcp_jsonrpc_stream(stream);
                });
            }
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(TCP_ACCEPT_IDLE_SLEEP_MILLIS));
            }
            Err(_) => return,
        }
    }
}

fn serve_tcp_jsonrpc_stream(stream: TcpStream) -> io::Result<()> {
    let reader = BufReader::new(stream.try_clone()?);
    let writer = BufWriter::new(stream);
    run_jsonrpc_lines(reader, writer)
}

fn validate_tcp_endpoint<F>(
    listen: &str,
    token_env: Option<&str>,
    token_lookup: &F,
) -> Result<McpEndpoint, McpTransportError>
where
    F: Fn(&str) -> Option<String>,
{
    let bind_addr = listen
        .parse::<SocketAddr>()
        .map_err(|_| McpTransportError::MalformedListenAddress(listen.to_owned()))?;

    let auth = if bind_addr.ip().is_loopback() {
        TcpAuth::NotRequiredForLoopback
    } else {
        let token_env = normalized_token_env(token_env, bind_addr)?;
        validate_token_value(&token_env, token_lookup)?;
        TcpAuth::TokenEnv { name: token_env }
    };

    Ok(McpEndpoint::Tcp { bind_addr, auth })
}

fn normalized_token_env(
    token_env: Option<&str>,
    bind_addr: SocketAddr,
) -> Result<String, McpTransportError> {
    let Some(token_env) = token_env else {
        return Err(McpTransportError::MissingTokenEnvForNonLoopback { bind_addr });
    };
    let token_env = token_env.trim();
    if token_env.is_empty() {
        return Err(McpTransportError::InvalidTokenEnvName {
            reason: REASON_EMPTY_TOKEN_ENV_NAME,
        });
    }

    Ok(token_env.to_owned())
}

fn validate_token_value<F>(token_env: &str, token_lookup: &F) -> Result<(), McpTransportError>
where
    F: Fn(&str) -> Option<String>,
{
    let Some(value) = token_lookup(token_env) else {
        return Err(McpTransportError::MissingTokenValue {
            token_env: token_env.to_owned(),
        });
    };
    if value.trim().is_empty() {
        return Err(McpTransportError::InvalidTokenValue {
            token_env: token_env.to_owned(),
            reason: REASON_EMPTY_TOKEN_VALUE,
        });
    }

    Ok(())
}

fn jsonrpc_result(id: Value, result: Value) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
    .to_string()
}

fn jsonrpc_error(id: Value, code: i64, message: &str) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
        },
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const LOOPBACK_LISTEN: &str = "127.0.0.1:4700";
    const LOOPBACK_EPHEMERAL_LISTEN: &str = "127.0.0.1:0";
    const IPV6_LOOPBACK_LISTEN: &str = "[::1]:4700";
    const NON_LOOPBACK_LISTEN: &str = "0.0.0.0:4700";
    const MALFORMED_LISTEN: &str = "not-a-socket";
    const TOKEN_ENV_NAME: &str = "STEVENARELLA_TEST_MCP_TOKEN";
    const TOKEN_VALUE: &str = "secret-token";

    #[test]
    fn stdio_transport_is_accepted_and_requires_clean_stdout() {
        let options = McpTransportOptions::from_cli(true, None, None);

        let validated = validate_transport_options(&options, |_| None).unwrap();

        assert_eq!(validated.endpoints, vec![McpEndpoint::Stdio]);
        assert!(validated.stdout_must_remain_clean);
    }

    #[test]
    fn loopback_tcp_transport_is_accepted_without_token() {
        let options = McpTransportOptions::from_cli(false, Some(LOOPBACK_LISTEN.to_owned()), None);

        let validated = validate_transport_options(&options, |_| None).unwrap();

        assert_eq!(
            validated.endpoints,
            vec![McpEndpoint::Tcp {
                bind_addr: LOOPBACK_LISTEN.parse().unwrap(),
                auth: TcpAuth::NotRequiredForLoopback,
            }]
        );
        assert!(!validated.stdout_must_remain_clean);
    }

    #[test]
    fn ipv6_loopback_tcp_transport_is_accepted_without_token() {
        let options =
            McpTransportOptions::from_cli(false, Some(IPV6_LOOPBACK_LISTEN.to_owned()), None);

        let validated = validate_transport_options(&options, |_| None).unwrap();

        assert_eq!(
            validated.endpoints,
            vec![McpEndpoint::Tcp {
                bind_addr: IPV6_LOOPBACK_LISTEN.parse().unwrap(),
                auth: TcpAuth::NotRequiredForLoopback,
            }]
        );
    }

    #[test]
    fn non_loopback_tcp_transport_is_rejected_without_token_env() {
        let options =
            McpTransportOptions::from_cli(false, Some(NON_LOOPBACK_LISTEN.to_owned()), None);

        assert_eq!(
            validate_transport_options(&options, |_| None),
            Err(McpTransportError::MissingTokenEnvForNonLoopback {
                bind_addr: NON_LOOPBACK_LISTEN.parse().unwrap(),
            })
        );
    }

    #[test]
    fn non_loopback_tcp_transport_is_rejected_with_empty_token_env_name() {
        let options = McpTransportOptions::from_cli(
            false,
            Some(NON_LOOPBACK_LISTEN.to_owned()),
            Some("  ".to_owned()),
        );

        assert_eq!(
            validate_transport_options(&options, |_| Some(TOKEN_VALUE.to_owned())),
            Err(McpTransportError::InvalidTokenEnvName {
                reason: REASON_EMPTY_TOKEN_ENV_NAME,
            })
        );
    }

    #[test]
    fn non_loopback_tcp_transport_is_rejected_with_missing_token_value() {
        let options = McpTransportOptions::from_cli(
            false,
            Some(NON_LOOPBACK_LISTEN.to_owned()),
            Some(TOKEN_ENV_NAME.to_owned()),
        );

        assert_eq!(
            validate_transport_options(&options, |_| None),
            Err(McpTransportError::MissingTokenValue {
                token_env: TOKEN_ENV_NAME.to_owned(),
            })
        );
    }

    #[test]
    fn non_loopback_tcp_transport_is_rejected_with_empty_token_value() {
        let options = McpTransportOptions::from_cli(
            false,
            Some(NON_LOOPBACK_LISTEN.to_owned()),
            Some(TOKEN_ENV_NAME.to_owned()),
        );

        assert_eq!(
            validate_transport_options(&options, |_| Some("  ".to_owned())),
            Err(McpTransportError::InvalidTokenValue {
                token_env: TOKEN_ENV_NAME.to_owned(),
                reason: REASON_EMPTY_TOKEN_VALUE,
            })
        );
    }

    #[test]
    fn non_loopback_tcp_transport_is_accepted_with_token_env_and_value() {
        let options = McpTransportOptions::from_cli(
            false,
            Some(NON_LOOPBACK_LISTEN.to_owned()),
            Some(format!(" {TOKEN_ENV_NAME} ")),
        );

        let validated = validate_transport_options(&options, |name| {
            assert_eq!(name, TOKEN_ENV_NAME);
            Some(TOKEN_VALUE.to_owned())
        })
        .unwrap();

        assert_eq!(
            validated.endpoints,
            vec![McpEndpoint::Tcp {
                bind_addr: NON_LOOPBACK_LISTEN.parse().unwrap(),
                auth: TcpAuth::TokenEnv {
                    name: TOKEN_ENV_NAME.to_owned(),
                },
            }]
        );
    }

    #[test]
    fn malformed_listen_address_is_rejected() {
        let options = McpTransportOptions::from_cli(false, Some(MALFORMED_LISTEN.to_owned()), None);

        assert_eq!(
            validate_transport_options(&options, |_| None),
            Err(McpTransportError::MalformedListenAddress(
                MALFORMED_LISTEN.to_owned()
            ))
        );
    }

    #[test]
    fn stdio_transport_runs_jsonrpc_line_loop() {
        let validated = ValidatedMcpTransport {
            endpoints: vec![McpEndpoint::Stdio],
            stdout_must_remain_clean: true,
        };
        let input = br#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}
"#;
        let output = Vec::new();

        let runtime = start_transport_with_stdio(validated, &input[..], output).unwrap();

        assert_eq!(runtime.endpoints, vec![StartedMcpEndpoint::Stdio]);
        assert!(runtime.stdout_must_remain_clean);
        assert_eq!(runtime.join_handle_count(), 1);
    }

    #[test]
    fn tcp_transport_binds_loopback_and_serves_jsonrpc() {
        let options =
            McpTransportOptions::from_cli(false, Some(LOOPBACK_EPHEMERAL_LISTEN.to_owned()), None);
        let validated = validate_transport_options(&options, |_| None).unwrap();

        let runtime = start_transport_runtime(validated).unwrap();
        let StartedMcpEndpoint::Tcp { local_addr } = runtime.endpoints[0] else {
            panic!("expected tcp endpoint");
        };

        let mut stream = TcpStream::connect(local_addr).unwrap();
        stream
            .write_all(
                br#"{"jsonrpc":"2.0","id":7,"method":"ping"}
"#,
            )
            .unwrap();
        let mut response = String::new();
        BufReader::new(stream).read_line(&mut response).unwrap();

        assert!(response.contains(r#""id":7"#));
        assert!(response.contains(r#""result":{}"#));
    }

    #[test]
    fn jsonrpc_handler_supports_initialize_and_lists_empty_tools() {
        let initialize = handle_jsonrpc_line(
            r#"{"jsonrpc":"2.0","id":"init","method":"initialize","params":{}}"#,
        )
        .unwrap();
        assert!(initialize.contains(r#""serverInfo"#));
        assert!(initialize.contains(r#""stevenarella"#));

        let tools =
            handle_jsonrpc_line(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#).unwrap();
        assert!(tools.contains(r#""tools":[]"#));
    }

    #[test]
    fn jsonrpc_handler_rejects_malformed_json() {
        let response = handle_jsonrpc_line("not-json").unwrap();

        assert!(response.contains(&JSONRPC_PARSE_ERROR.to_string()));
    }
}
