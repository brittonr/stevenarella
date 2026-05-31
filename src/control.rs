// Copyright 2026
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your option.

use serde_json::{Map, Value};

pub const MAX_SERVER_ADDRESS_CHARS: usize = 255;
pub const MAX_CHAT_MESSAGE_CHARS: usize = 256;
pub const MAX_ABSOLUTE_LOOK_DELTA_RADIANS: f64 = std::f64::consts::PI;

const FIELD_ACTION: &str = "action";
const FIELD_ADDRESS: &str = "address";
const FIELD_BUTTON: &str = "button";
const FIELD_DOWN: &str = "down";
const FIELD_KEY: &str = "key";
const FIELD_MESSAGE: &str = "message";
const FIELD_PITCH_DELTA: &str = "pitch_delta";
const FIELD_YAW_DELTA: &str = "yaw_delta";

const ACTION_STATUS: &str = "status";
const ACTION_CONNECT: &str = "connect";
const ACTION_DISCONNECT: &str = "disconnect";
const ACTION_KEY: &str = "key";
const ACTION_LOOK: &str = "look";
const ACTION_MOUSE: &str = "mouse";
const ACTION_USE_ITEM: &str = "use_item";
const ACTION_USE_ITEM_ALIAS: &str = "use-item";
const ACTION_ATTACK: &str = "attack";
const ACTION_CHAT: &str = "chat";

const KEY_FORWARD: &str = "forward";
const KEY_BACKWARD: &str = "backward";
const KEY_LEFT: &str = "left";
const KEY_RIGHT: &str = "right";
const KEY_OPEN_INVENTORY: &str = "open_inventory";
const KEY_OPEN_INVENTORY_ALIAS: &str = "open-inventory";
const KEY_OPEN_INV: &str = "open_inv";
const KEY_OPEN_INV_ALIAS: &str = "open-inv";
const KEY_SNEAK: &str = "sneak";
const KEY_SPRINT: &str = "sprint";
const KEY_JUMP: &str = "jump";

const BUTTON_LEFT: &str = "left";
const BUTTON_RIGHT: &str = "right";

const REASON_EMPTY: &str = "empty";
const REASON_EMPTY_OR_WHITESPACE: &str = "empty_or_whitespace";
const REASON_EXPECTED_BOOL: &str = "expected_bool";
const REASON_EXPECTED_NUMBER: &str = "expected_number";
const REASON_EXPECTED_STRING: &str = "expected_string";
const REASON_NOT_FINITE: &str = "not_finite";

#[derive(Debug, Clone, PartialEq)]
pub enum ControlCommand {
    Status,
    Connect { address: String },
    Disconnect,
    Key { key: ControlKey, down: bool },
    Look { yaw_delta: f64, pitch_delta: f64 },
    Mouse { button: MouseButton, down: bool },
    UseItem,
    Attack,
    Chat { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlKey {
    Forward,
    Backward,
    Left,
    Right,
    OpenInventory,
    Sneak,
    Sprint,
    Jump,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlOutcome {
    Applied,
    Rejected,
    Deferred,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlResponse {
    pub outcome: ControlOutcome,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ControlError {
    MalformedJson(String),
    ExpectedObject,
    MissingField(&'static str),
    InvalidField {
        field: &'static str,
        reason: &'static str,
    },
    UnknownAction(String),
    UnknownKey(String),
    UnknownMouseButton(String),
    ValueTooLong {
        field: &'static str,
        max_chars: usize,
        actual_chars: usize,
    },
    OutOfRange {
        field: &'static str,
        max_abs: f64,
        actual: f64,
    },
}

pub fn parse_control_command(input: &str) -> Result<ControlCommand, ControlError> {
    let value: Value =
        serde_json::from_str(input).map_err(|err| ControlError::MalformedJson(err.to_string()))?;
    parse_control_command_value(&value)
}

pub fn parse_control_command_value(value: &Value) -> Result<ControlCommand, ControlError> {
    let object = value.as_object().ok_or(ControlError::ExpectedObject)?;
    let action = required_string(object, FIELD_ACTION)?;

    match action {
        ACTION_STATUS => Ok(ControlCommand::Status),
        ACTION_CONNECT => parse_connect(object),
        ACTION_DISCONNECT => Ok(ControlCommand::Disconnect),
        ACTION_KEY => parse_key_command(object),
        ACTION_LOOK => parse_look(object),
        ACTION_MOUSE => parse_mouse(object),
        ACTION_USE_ITEM | ACTION_USE_ITEM_ALIAS => Ok(ControlCommand::UseItem),
        ACTION_ATTACK => Ok(ControlCommand::Attack),
        ACTION_CHAT => parse_chat(object),
        _ => Err(ControlError::UnknownAction(action.to_owned())),
    }
}

impl ControlKey {
    pub fn from_name(name: &str) -> Result<Self, ControlError> {
        match name {
            KEY_FORWARD => Ok(ControlKey::Forward),
            KEY_BACKWARD => Ok(ControlKey::Backward),
            KEY_LEFT => Ok(ControlKey::Left),
            KEY_RIGHT => Ok(ControlKey::Right),
            KEY_OPEN_INVENTORY | KEY_OPEN_INVENTORY_ALIAS | KEY_OPEN_INV | KEY_OPEN_INV_ALIAS => {
                Ok(ControlKey::OpenInventory)
            }
            KEY_SNEAK => Ok(ControlKey::Sneak),
            KEY_SPRINT => Ok(ControlKey::Sprint),
            KEY_JUMP => Ok(ControlKey::Jump),
            _ => Err(ControlError::UnknownKey(name.to_owned())),
        }
    }
}

impl MouseButton {
    pub fn from_name(name: &str) -> Result<Self, ControlError> {
        match name {
            BUTTON_LEFT => Ok(MouseButton::Left),
            BUTTON_RIGHT => Ok(MouseButton::Right),
            _ => Err(ControlError::UnknownMouseButton(name.to_owned())),
        }
    }
}

fn parse_connect(object: &Map<String, Value>) -> Result<ControlCommand, ControlError> {
    let raw_address = required_string(object, FIELD_ADDRESS)?;
    let address = raw_address.trim();
    validate_nonempty_string(FIELD_ADDRESS, address)?;
    validate_max_chars(FIELD_ADDRESS, address, MAX_SERVER_ADDRESS_CHARS)?;
    Ok(ControlCommand::Connect {
        address: address.to_owned(),
    })
}

fn parse_key_command(object: &Map<String, Value>) -> Result<ControlCommand, ControlError> {
    let key = ControlKey::from_name(required_string(object, FIELD_KEY)?)?;
    let down = required_bool(object, FIELD_DOWN)?;
    Ok(ControlCommand::Key { key, down })
}

fn parse_look(object: &Map<String, Value>) -> Result<ControlCommand, ControlError> {
    let yaw_delta = required_bounded_f64(object, FIELD_YAW_DELTA)?;
    let pitch_delta = required_bounded_f64(object, FIELD_PITCH_DELTA)?;
    Ok(ControlCommand::Look {
        yaw_delta,
        pitch_delta,
    })
}

fn parse_mouse(object: &Map<String, Value>) -> Result<ControlCommand, ControlError> {
    let button = MouseButton::from_name(required_string(object, FIELD_BUTTON)?)?;
    let down = required_bool(object, FIELD_DOWN)?;
    Ok(ControlCommand::Mouse { button, down })
}

fn parse_chat(object: &Map<String, Value>) -> Result<ControlCommand, ControlError> {
    let message = required_string(object, FIELD_MESSAGE)?;
    validate_nonblank_string(FIELD_MESSAGE, message)?;
    validate_max_chars(FIELD_MESSAGE, message, MAX_CHAT_MESSAGE_CHARS)?;
    Ok(ControlCommand::Chat {
        message: message.to_owned(),
    })
}

fn required_string<'a>(
    object: &'a Map<String, Value>,
    field: &'static str,
) -> Result<&'a str, ControlError> {
    object
        .get(field)
        .ok_or(ControlError::MissingField(field))?
        .as_str()
        .ok_or(ControlError::InvalidField {
            field,
            reason: REASON_EXPECTED_STRING,
        })
}

fn required_bool(object: &Map<String, Value>, field: &'static str) -> Result<bool, ControlError> {
    object
        .get(field)
        .ok_or(ControlError::MissingField(field))?
        .as_bool()
        .ok_or(ControlError::InvalidField {
            field,
            reason: REASON_EXPECTED_BOOL,
        })
}

fn required_bounded_f64(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<f64, ControlError> {
    let value = object
        .get(field)
        .ok_or(ControlError::MissingField(field))?
        .as_f64()
        .ok_or(ControlError::InvalidField {
            field,
            reason: REASON_EXPECTED_NUMBER,
        })?;

    if !value.is_finite() {
        return Err(ControlError::InvalidField {
            field,
            reason: REASON_NOT_FINITE,
        });
    }

    if value.abs() > MAX_ABSOLUTE_LOOK_DELTA_RADIANS {
        return Err(ControlError::OutOfRange {
            field,
            max_abs: MAX_ABSOLUTE_LOOK_DELTA_RADIANS,
            actual: value,
        });
    }

    Ok(value)
}

fn validate_nonempty_string(field: &'static str, value: &str) -> Result<(), ControlError> {
    if value.is_empty() {
        return Err(ControlError::InvalidField {
            field,
            reason: REASON_EMPTY,
        });
    }

    Ok(())
}

fn validate_nonblank_string(field: &'static str, value: &str) -> Result<(), ControlError> {
    if value.trim().is_empty() {
        return Err(ControlError::InvalidField {
            field,
            reason: REASON_EMPTY_OR_WHITESPACE,
        });
    }

    Ok(())
}

fn validate_max_chars(
    field: &'static str,
    value: &str,
    max_chars: usize,
) -> Result<(), ControlError> {
    let actual_chars = value.chars().count();
    if actual_chars > max_chars {
        return Err(ControlError::ValueTooLong {
            field,
            max_chars,
            actual_chars,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const VALID_LOOK_YAW_DELTA: f64 = 0.25;
    const VALID_LOOK_PITCH_DELTA: f64 = -0.125;
    const OVERSIZED_CHAT_EXTRA_CHARS: usize = 1;

    #[test]
    fn parses_valid_initial_command_set() {
        assert_eq!(
            parse_control_command_value(&json!({ "action": "status" })),
            Ok(ControlCommand::Status)
        );
        assert_eq!(
            parse_control_command_value(
                &json!({ "action": "connect", "address": " 127.0.0.1:25565 " })
            ),
            Ok(ControlCommand::Connect {
                address: "127.0.0.1:25565".to_owned(),
            })
        );
        assert_eq!(
            parse_control_command_value(&json!({ "action": "disconnect" })),
            Ok(ControlCommand::Disconnect)
        );
        assert_eq!(
            parse_control_command_value(
                &json!({ "action": "key", "key": "forward", "down": true })
            ),
            Ok(ControlCommand::Key {
                key: ControlKey::Forward,
                down: true,
            })
        );
        assert_eq!(
            parse_control_command_value(&json!({
                "action": "look",
                "yaw_delta": VALID_LOOK_YAW_DELTA,
                "pitch_delta": VALID_LOOK_PITCH_DELTA,
            })),
            Ok(ControlCommand::Look {
                yaw_delta: VALID_LOOK_YAW_DELTA,
                pitch_delta: VALID_LOOK_PITCH_DELTA,
            })
        );
        assert_eq!(
            parse_control_command_value(
                &json!({ "action": "mouse", "button": "right", "down": false })
            ),
            Ok(ControlCommand::Mouse {
                button: MouseButton::Right,
                down: false,
            })
        );
        assert_eq!(
            parse_control_command_value(&json!({ "action": "use-item" })),
            Ok(ControlCommand::UseItem)
        );
        assert_eq!(
            parse_control_command_value(&json!({ "action": "attack" })),
            Ok(ControlCommand::Attack)
        );
        assert_eq!(
            parse_control_command_value(&json!({ "action": "chat", "message": "/help" })),
            Ok(ControlCommand::Chat {
                message: "/help".to_owned(),
            })
        );
    }

    #[test]
    fn parses_all_key_aliases() {
        assert_eq!(ControlKey::from_name("backward"), Ok(ControlKey::Backward));
        assert_eq!(ControlKey::from_name("left"), Ok(ControlKey::Left));
        assert_eq!(ControlKey::from_name("right"), Ok(ControlKey::Right));
        assert_eq!(
            ControlKey::from_name("open_inventory"),
            Ok(ControlKey::OpenInventory)
        );
        assert_eq!(
            ControlKey::from_name("open-inventory"),
            Ok(ControlKey::OpenInventory)
        );
        assert_eq!(
            ControlKey::from_name("open_inv"),
            Ok(ControlKey::OpenInventory)
        );
        assert_eq!(
            ControlKey::from_name("open-inv"),
            Ok(ControlKey::OpenInventory)
        );
        assert_eq!(ControlKey::from_name("sneak"), Ok(ControlKey::Sneak));
        assert_eq!(ControlKey::from_name("sprint"), Ok(ControlKey::Sprint));
        assert_eq!(ControlKey::from_name("jump"), Ok(ControlKey::Jump));
    }

    #[test]
    fn rejects_missing_action_field() {
        assert_eq!(
            parse_control_command_value(&json!({ "key": "forward", "down": true })),
            Err(ControlError::MissingField(FIELD_ACTION))
        );
    }

    #[test]
    fn rejects_unknown_action() {
        assert_eq!(
            parse_control_command_value(&json!({ "action": "teleport" })),
            Err(ControlError::UnknownAction("teleport".to_owned()))
        );
    }

    #[test]
    fn rejects_invalid_key_name() {
        assert_eq!(
            parse_control_command_value(&json!({ "action": "key", "key": "fly", "down": true })),
            Err(ControlError::UnknownKey("fly".to_owned()))
        );
    }

    #[test]
    fn rejects_invalid_mouse_button() {
        assert_eq!(
            parse_control_command_value(
                &json!({ "action": "mouse", "button": "middle", "down": true })
            ),
            Err(ControlError::UnknownMouseButton("middle".to_owned()))
        );
    }

    #[test]
    fn rejects_missing_required_command_field() {
        assert_eq!(
            parse_control_command_value(&json!({ "action": "key", "key": "jump" })),
            Err(ControlError::MissingField(FIELD_DOWN))
        );
    }

    #[test]
    fn rejects_oversized_chat_message() {
        let oversized = "x".repeat(MAX_CHAT_MESSAGE_CHARS + OVERSIZED_CHAT_EXTRA_CHARS);
        assert_eq!(
            parse_control_command_value(&json!({ "action": "chat", "message": oversized })),
            Err(ControlError::ValueTooLong {
                field: FIELD_MESSAGE,
                max_chars: MAX_CHAT_MESSAGE_CHARS,
                actual_chars: MAX_CHAT_MESSAGE_CHARS + OVERSIZED_CHAT_EXTRA_CHARS,
            })
        );
    }

    #[test]
    fn rejects_blank_chat_message() {
        assert_eq!(
            parse_control_command_value(&json!({ "action": "chat", "message": "   " })),
            Err(ControlError::InvalidField {
                field: FIELD_MESSAGE,
                reason: REASON_EMPTY_OR_WHITESPACE,
            })
        );
    }

    #[test]
    fn rejects_out_of_range_look_delta() {
        assert_eq!(
            parse_control_command_value(&json!({
                "action": "look",
                "yaw_delta": MAX_ABSOLUTE_LOOK_DELTA_RADIANS * 2.0,
                "pitch_delta": VALID_LOOK_PITCH_DELTA,
            })),
            Err(ControlError::OutOfRange {
                field: FIELD_YAW_DELTA,
                max_abs: MAX_ABSOLUTE_LOOK_DELTA_RADIANS,
                actual: MAX_ABSOLUTE_LOOK_DELTA_RADIANS * 2.0,
            })
        );
    }

    #[test]
    fn parses_json_string_entrypoint() {
        assert_eq!(
            parse_control_command(r#"{"action":"mouse","button":"left","down":true}"#),
            Ok(ControlCommand::Mouse {
                button: MouseButton::Left,
                down: true,
            })
        );
    }

    #[test]
    fn rejects_malformed_json_entrypoint() {
        match parse_control_command("not-json") {
            Err(ControlError::MalformedJson(_)) => {}
            other => panic!("expected malformed json, got {:?}", other),
        }
    }
}
