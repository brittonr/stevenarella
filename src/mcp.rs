// Copyright 2026
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your option.

use std::net::SocketAddr;

pub const DEFAULT_MCP_TOKEN_ENV: &str = "STEVENARELLA_MCP_TOKEN";

const REASON_EMPTY_TOKEN_ENV_NAME: &str = "empty_token_env_name";
const REASON_EMPTY_TOKEN_VALUE: &str = "empty_token_value";

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

pub fn start_transport_stub(validated: ValidatedMcpTransport) -> McpTransportRuntime {
    McpTransportRuntime {
        endpoints: validated.endpoints,
        stdout_must_remain_clean: validated.stdout_must_remain_clean,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpTransportRuntime {
    pub endpoints: Vec<McpEndpoint>,
    pub stdout_must_remain_clean: bool,
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

#[cfg(test)]
mod tests {
    use super::*;

    const LOOPBACK_LISTEN: &str = "127.0.0.1:4700";
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
    fn transport_stub_preserves_validated_endpoint_shape() {
        let options = McpTransportOptions::from_cli(true, Some(LOOPBACK_LISTEN.to_owned()), None);
        let validated = validate_transport_options(&options, |_| None).unwrap();

        let runtime = start_transport_stub(validated);

        assert_eq!(runtime.endpoints.len(), 2);
        assert!(runtime.stdout_must_remain_clean);
    }
}
