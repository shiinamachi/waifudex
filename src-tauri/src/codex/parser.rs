use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    TaskStarted,
    TaskCompleted,
    ToolCallStarted { tool_name: String },
    ToolCallCompleted,
    MessageDelta,
    Error { message: String },
    TokenCount,
    Unknown,
}

pub fn parse_session_line(line: &str) -> SessionEvent {
    let value = match serde_json::from_str::<Value>(line) {
        Ok(value) => value,
        Err(_) => return SessionEvent::Unknown,
    };

    let event_name = read_event_name(&value);

    if event_name == "task_started" {
        return SessionEvent::TaskStarted;
    }
    if event_name == "task_complete" {
        return SessionEvent::TaskCompleted;
    }
    if event_name == "function_call" {
        return SessionEvent::ToolCallStarted {
            tool_name: read_tool_name(&value),
        };
    }
    if event_name == "function_call_output" || event_name == "custom_tool_call_output" {
        return SessionEvent::ToolCallCompleted;
    }
    if matches!(
        event_name.as_str(),
        "agent_message" | "reasoning" | "message" | "output_text"
    ) {
        return SessionEvent::MessageDelta;
    }
    if event_name == "token_count" {
        return SessionEvent::TokenCount;
    }
    if event_name.contains("error") || has_error(&value) {
        return SessionEvent::Error {
            message: read_error_message(&value),
        };
    }

    SessionEvent::Unknown
}

fn read_event_name(value: &Value) -> String {
    if let Some(payload_type) = value
        .get("payload")
        .and_then(|payload| payload.get("type"))
        .and_then(Value::as_str)
    {
        return payload_type.to_ascii_lowercase();
    }

    for key in ["event", "type", "kind"] {
        if let Some(name) = value.get(key).and_then(Value::as_str) {
            return name.to_ascii_lowercase();
        }
    }

    String::new()
}

fn read_tool_name(value: &Value) -> String {
    if let Some(tool_name) = value
        .get("payload")
        .and_then(|payload| payload.get("name"))
        .and_then(Value::as_str)
    {
        return tool_name.to_string();
    }
    if let Some(tool_name) = value.get("tool_name").and_then(Value::as_str) {
        return tool_name.to_string();
    }
    if let Some(tool_name) = value
        .get("tool")
        .and_then(|tool| tool.get("name"))
        .and_then(Value::as_str)
    {
        return tool_name.to_string();
    }

    "unknown_tool".to_string()
}

fn read_error_message(value: &Value) -> String {
    if let Some(message) = value
        .get("payload")
        .and_then(|payload| payload.get("message"))
        .and_then(Value::as_str)
    {
        return message.to_string();
    }
    if let Some(message) = value.get("message").and_then(Value::as_str) {
        return message.to_string();
    }
    if let Some(message) = value
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
    {
        return message.to_string();
    }

    "unknown error".to_string()
}

fn has_error(value: &Value) -> bool {
    value.get("error").is_some()
        || value
            .get("payload")
            .and_then(|payload| payload.get("error"))
            .is_some()
}

#[cfg(test)]
mod parser_tests {
    use super::{parse_session_line, SessionEvent};

    #[test]
    fn parses_task_started_from_event_msg_payload() {
        let line = r#"{"type":"event_msg","payload":{"type":"task_started","turn_id":"abc"}}"#;
        let event = parse_session_line(line);

        assert_eq!(event, SessionEvent::TaskStarted);
    }

    #[test]
    fn parses_function_call_from_response_item_payload() {
        let line = r#"{"type":"response_item","payload":{"type":"function_call","name":"exec_command","arguments":"{}","call_id":"call_123"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::ToolCallStarted {
                tool_name: "exec_command".to_string(),
            }
        );
    }

    #[test]
    fn parses_task_complete_from_event_msg_payload() {
        let line = r#"{"type":"event_msg","payload":{"type":"task_complete","turn_id":"abc"}}"#;
        let event = parse_session_line(line);

        assert_eq!(event, SessionEvent::TaskCompleted);
    }

    #[test]
    fn parses_function_call_output_from_response_item_payload() {
        let line = r#"{"type":"response_item","payload":{"type":"function_call_output","call_id":"call_123","output":"ok"}}"#;
        let event = parse_session_line(line);

        assert_eq!(event, SessionEvent::ToolCallCompleted);
    }

    #[test]
    fn parses_error_event_with_message() {
        let line =
            r#"{"type":"event_msg","payload":{"type":"error","message":"permission denied"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::Error {
                message: "permission denied".to_string(),
            }
        );
    }

    #[test]
    fn maps_unknown_event_to_unknown() {
        let line = r#"{"type":"response_item","payload":{"type":"something_weird"}}"#;
        let event = parse_session_line(line);

        assert_eq!(event, SessionEvent::Unknown);
    }
}
