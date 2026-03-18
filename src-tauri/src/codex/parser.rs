use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionKind {
    ExecApproval,
    RequestPermissions,
    RequestUserInput,
    Elicitation,
    PatchApproval,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    TaskStarted {
        event_name: String,
    },
    TaskCompleted {
        event_name: String,
    },
    ToolCallStarted {
        event_name: String,
        call_id: Option<String>,
        tool_name: String,
    },
    ToolCallCompleted {
        event_name: String,
        call_id: Option<String>,
    },
    CodingStarted {
        event_name: String,
        call_id: Option<String>,
        tool_name: String,
    },
    CodingCompleted {
        event_name: String,
        call_id: Option<String>,
    },
    QuestionAsked {
        event_name: String,
        call_id: Option<String>,
        kind: QuestionKind,
    },
    TurnAborted {
        event_name: String,
    },
    MessageDelta {
        event_name: String,
    },
    Error {
        event_name: String,
        message: String,
    },
    TokenCount {
        event_name: String,
    },
    Unknown {
        event_name: Option<String>,
    },
}

impl SessionEvent {
    pub fn parsed_type(&self) -> Option<&str> {
        match self {
            SessionEvent::TaskStarted { event_name }
            | SessionEvent::TaskCompleted { event_name }
            | SessionEvent::ToolCallStarted { event_name, .. }
            | SessionEvent::ToolCallCompleted { event_name, .. }
            | SessionEvent::CodingStarted { event_name, .. }
            | SessionEvent::CodingCompleted { event_name, .. }
            | SessionEvent::QuestionAsked { event_name, .. }
            | SessionEvent::TurnAborted { event_name }
            | SessionEvent::MessageDelta { event_name }
            | SessionEvent::Error { event_name, .. }
            | SessionEvent::TokenCount { event_name } => Some(event_name.as_str()),
            SessionEvent::Unknown { event_name } => event_name.as_deref(),
        }
    }

    pub fn parse_ok(&self) -> bool {
        !matches!(self, SessionEvent::Unknown { .. })
    }
}

pub fn parse_session_line(line: &str) -> SessionEvent {
    let value = match serde_json::from_str::<Value>(line) {
        Ok(value) => value,
        Err(_) => return SessionEvent::Unknown { event_name: None },
    };

    let event_name = read_event_name(&value);
    let call_id = read_call_id(&value);

    if matches!(event_name.as_str(), "task_started" | "turn_started") {
        return SessionEvent::TaskStarted { event_name };
    }
    if matches!(event_name.as_str(), "task_complete" | "turn_complete") {
        return SessionEvent::TaskCompleted { event_name };
    }
    if event_name == "turn_aborted" {
        return SessionEvent::TurnAborted { event_name };
    }
    if matches!(
        event_name.as_str(),
        "exec_approval_request"
            | "request_permissions"
            | "request_user_input"
            | "elicitation_request"
            | "apply_patch_approval_request"
    ) {
        return SessionEvent::QuestionAsked {
            kind: classify_question_kind(&event_name),
            event_name,
            call_id,
        };
    }
    if event_name == "patch_apply_begin" {
        return SessionEvent::CodingStarted {
            event_name,
            call_id,
            tool_name: "apply_patch".to_string(),
        };
    }
    if event_name == "patch_apply_end" {
        return SessionEvent::CodingCompleted {
            event_name,
            call_id,
        };
    }
    if event_name == "exec_command_begin" {
        if read_command_text(&value).is_some_and(|command| command_looks_mutating(&command)) {
            return SessionEvent::CodingStarted {
                event_name,
                call_id,
                tool_name: "exec_command".to_string(),
            };
        }
        return SessionEvent::ToolCallStarted {
            event_name,
            call_id,
            tool_name: "exec_command".to_string(),
        };
    }
    if event_name == "exec_command_end" {
        return SessionEvent::ToolCallCompleted {
            event_name,
            call_id,
        };
    }
    if event_name == "web_search_begin" {
        return SessionEvent::ToolCallStarted {
            event_name,
            call_id,
            tool_name: "web_search".to_string(),
        };
    }
    if event_name == "web_search_end" {
        return SessionEvent::ToolCallCompleted {
            event_name,
            call_id,
        };
    }
    if event_name == "function_call" {
        let tool_name = read_tool_name(&value);
        if read_command_text(&value).is_some_and(|command| command_looks_mutating(&command)) {
            return SessionEvent::CodingStarted {
                event_name,
                call_id,
                tool_name,
            };
        }
        return SessionEvent::ToolCallStarted {
            event_name,
            call_id,
            tool_name,
        };
    }
    if event_name == "custom_tool_call" {
        let tool_name = read_tool_name(&value);
        if tool_name == "apply_patch" {
            return SessionEvent::CodingStarted {
                event_name,
                call_id,
                tool_name,
            };
        }
        return SessionEvent::ToolCallStarted {
            event_name,
            call_id,
            tool_name,
        };
    }
    if event_name == "local_shell_call" {
        let is_coding =
            read_command_text(&value).is_some_and(|command| command_looks_mutating(&command));
        let is_complete = read_status(&value)
            .is_some_and(|status| matches!(status.as_str(), "completed" | "incomplete"));
        if is_complete {
            if is_coding {
                return SessionEvent::CodingCompleted {
                    event_name,
                    call_id,
                };
            }
            return SessionEvent::ToolCallCompleted {
                event_name,
                call_id,
            };
        }
        if is_coding {
            return SessionEvent::CodingStarted {
                event_name,
                call_id,
                tool_name: "local_shell_call".to_string(),
            };
        }
        return SessionEvent::ToolCallStarted {
            event_name,
            call_id,
            tool_name: "local_shell_call".to_string(),
        };
    }
    if matches!(
        event_name.as_str(),
        "function_call_output" | "custom_tool_call_output"
    ) {
        return SessionEvent::ToolCallCompleted {
            event_name,
            call_id,
        };
    }
    if matches!(
        event_name.as_str(),
        "agent_message" | "reasoning" | "message" | "output_text" | "user_message"
    ) {
        return SessionEvent::MessageDelta { event_name };
    }
    if event_name == "token_count" {
        return SessionEvent::TokenCount { event_name };
    }
    if event_name.contains("error") || has_error(&value) {
        return SessionEvent::Error {
            event_name,
            message: read_error_message(&value),
        };
    }

    SessionEvent::Unknown {
        event_name: Some(event_name),
    }
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

fn read_call_id(value: &Value) -> Option<String> {
    value
        .get("payload")
        .and_then(|payload| payload.get("call_id"))
        .and_then(Value::as_str)
        .or_else(|| value.get("call_id").and_then(Value::as_str))
        .or_else(|| {
            value
                .get("payload")
                .and_then(|payload| payload.get("id"))
                .and_then(Value::as_str)
        })
        .or_else(|| value.get("id").and_then(Value::as_str))
        .map(ToString::to_string)
}

fn read_status(value: &Value) -> Option<String> {
    value
        .get("payload")
        .and_then(|payload| payload.get("status"))
        .and_then(Value::as_str)
        .or_else(|| value.get("status").and_then(Value::as_str))
        .map(|status| status.to_ascii_lowercase())
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

fn read_command_text(value: &Value) -> Option<String> {
    read_command_value(
        value
            .get("payload")
            .and_then(|payload| payload.get("command"))
            .or_else(|| value.get("command")),
    )
    .or_else(|| {
        read_command_value(
            value
                .get("payload")
                .and_then(|payload| payload.get("action"))
                .and_then(|action| action.get("command")),
        )
    })
    .or_else(|| {
        let arguments = value
            .get("payload")
            .and_then(|payload| payload.get("arguments"))
            .and_then(Value::as_str)?;
        let parsed = serde_json::from_str::<Value>(arguments).ok()?;
        read_command_value(parsed.get("command"))
    })
}

fn read_command_value(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(command) => Some(command.clone()),
        Value::Array(parts) => {
            let tokens = parts
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<String>>();
            if tokens.is_empty() {
                None
            } else {
                Some(tokens.join(" "))
            }
        }
        _ => None,
    }
}

fn command_looks_mutating(command: &str) -> bool {
    let normalized = command.to_ascii_lowercase();
    let tokens = normalized.split_whitespace().collect::<Vec<&str>>();
    let first = tokens.first().copied().unwrap_or_default();

    if matches!(
        first,
        "touch" | "mkdir" | "rm" | "mv" | "cp" | "install" | "truncate" | "tee"
    ) {
        return true;
    }

    if normalized.contains("sed -i")
        || normalized.contains("perl -pi")
        || normalized.contains(">>")
        || normalized.contains(" >")
        || normalized.starts_with('>')
        || normalized.contains(".write(")
        || normalized.contains("writefile")
        || normalized.contains("write_file")
        || normalized.contains("write_text")
        || normalized.contains("fs.write")
    {
        return true;
    }

    false
}

fn classify_question_kind(event_name: &str) -> QuestionKind {
    match event_name {
        "exec_approval_request" => QuestionKind::ExecApproval,
        "request_permissions" => QuestionKind::RequestPermissions,
        "request_user_input" => QuestionKind::RequestUserInput,
        "elicitation_request" => QuestionKind::Elicitation,
        "apply_patch_approval_request" => QuestionKind::PatchApproval,
        _ => unreachable!("unexpected question event: {event_name}"),
    }
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
    use super::{parse_session_line, QuestionKind, SessionEvent};

    #[test]
    fn parses_task_started_from_event_msg_payload() {
        let line = r#"{"type":"event_msg","payload":{"type":"task_started","turn_id":"abc"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::TaskStarted {
                event_name: "task_started".to_string(),
            }
        );
        assert_eq!(event.parsed_type(), Some("task_started"));
    }

    #[test]
    fn parses_turn_started_alias_from_event_msg_payload() {
        let line = r#"{"type":"event_msg","payload":{"type":"turn_started","turn_id":"abc"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::TaskStarted {
                event_name: "turn_started".to_string(),
            }
        );
        assert_eq!(event.parsed_type(), Some("turn_started"));
    }

    #[test]
    fn parses_user_message_as_message_delta_for_immediate_activity() {
        let line = r#"{"type":"event_msg","payload":{"type":"user_message","message":"hello"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::MessageDelta {
                event_name: "user_message".to_string(),
            }
        );
        assert_eq!(event.parsed_type(), Some("user_message"));
    }

    #[test]
    fn parses_function_call_from_response_item_payload() {
        let line = r#"{"type":"response_item","payload":{"type":"function_call","name":"exec_command","arguments":"{}","call_id":"call_123"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::ToolCallStarted {
                event_name: "function_call".to_string(),
                call_id: Some("call_123".to_string()),
                tool_name: "exec_command".to_string(),
            }
        );
        assert_eq!(event.parsed_type(), Some("function_call"));
    }

    #[test]
    fn parses_custom_tool_call_from_response_item_payload() {
        let line = r#"{"type":"response_item","payload":{"type":"custom_tool_call","status":"completed","call_id":"call_123","name":"apply_patch","input":"*** Begin Patch"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::CodingStarted {
                event_name: "custom_tool_call".to_string(),
                call_id: Some("call_123".to_string()),
                tool_name: "apply_patch".to_string(),
            }
        );
        assert_eq!(event.parsed_type(), Some("custom_tool_call"));
    }

    #[test]
    fn parses_exec_approval_request_event() {
        let line = r#"{"type":"event_msg","payload":{"type":"exec_approval_request","call_id":"call_456"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::QuestionAsked {
                event_name: "exec_approval_request".to_string(),
                call_id: Some("call_456".to_string()),
                kind: QuestionKind::ExecApproval,
            }
        );
        assert_eq!(event.parsed_type(), Some("exec_approval_request"));
    }

    #[test]
    fn parses_request_permissions_event() {
        let line =
            r#"{"type":"event_msg","payload":{"type":"request_permissions","call_id":"call_789"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::QuestionAsked {
                event_name: "request_permissions".to_string(),
                call_id: Some("call_789".to_string()),
                kind: QuestionKind::RequestPermissions,
            }
        );
        assert_eq!(event.parsed_type(), Some("request_permissions"));
    }

    #[test]
    fn parses_request_user_input_event() {
        let line =
            r#"{"type":"event_msg","payload":{"type":"request_user_input","call_id":"call_321"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::QuestionAsked {
                event_name: "request_user_input".to_string(),
                call_id: Some("call_321".to_string()),
                kind: QuestionKind::RequestUserInput,
            }
        );
        assert_eq!(event.parsed_type(), Some("request_user_input"));
    }

    #[test]
    fn parses_elicitation_request_event() {
        let line =
            r#"{"type":"event_msg","payload":{"type":"elicitation_request","call_id":"call_654"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::QuestionAsked {
                event_name: "elicitation_request".to_string(),
                call_id: Some("call_654".to_string()),
                kind: QuestionKind::Elicitation,
            }
        );
        assert_eq!(event.parsed_type(), Some("elicitation_request"));
    }

    #[test]
    fn parses_apply_patch_approval_request_event() {
        let line = r#"{"type":"event_msg","payload":{"type":"apply_patch_approval_request","call_id":"call_987"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::QuestionAsked {
                event_name: "apply_patch_approval_request".to_string(),
                call_id: Some("call_987".to_string()),
                kind: QuestionKind::PatchApproval,
            }
        );
        assert_eq!(event.parsed_type(), Some("apply_patch_approval_request"));
    }

    #[test]
    fn parses_task_complete_from_event_msg_payload() {
        let line = r#"{"type":"event_msg","payload":{"type":"task_complete","turn_id":"abc"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::TaskCompleted {
                event_name: "task_complete".to_string(),
            }
        );
        assert_eq!(event.parsed_type(), Some("task_complete"));
    }

    #[test]
    fn parses_turn_complete_alias_from_event_msg_payload() {
        let line = r#"{"type":"event_msg","payload":{"type":"turn_complete","turn_id":"abc"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::TaskCompleted {
                event_name: "turn_complete".to_string(),
            }
        );
        assert_eq!(event.parsed_type(), Some("turn_complete"));
    }

    #[test]
    fn parses_patch_apply_begin_event() {
        let line =
            r#"{"type":"event_msg","payload":{"type":"patch_apply_begin","call_id":"call_patch"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::CodingStarted {
                event_name: "patch_apply_begin".to_string(),
                call_id: Some("call_patch".to_string()),
                tool_name: "apply_patch".to_string(),
            }
        );
        assert_eq!(event.parsed_type(), Some("patch_apply_begin"));
    }

    #[test]
    fn parses_patch_apply_end_event() {
        let line =
            r#"{"type":"event_msg","payload":{"type":"patch_apply_end","call_id":"call_patch"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::CodingCompleted {
                event_name: "patch_apply_end".to_string(),
                call_id: Some("call_patch".to_string()),
            }
        );
        assert_eq!(event.parsed_type(), Some("patch_apply_end"));
    }

    #[test]
    fn parses_exec_command_begin_event() {
        let line =
            r#"{"type":"event_msg","payload":{"type":"exec_command_begin","call_id":"call_exec"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::ToolCallStarted {
                event_name: "exec_command_begin".to_string(),
                call_id: Some("call_exec".to_string()),
                tool_name: "exec_command".to_string(),
            }
        );
        assert_eq!(event.parsed_type(), Some("exec_command_begin"));
    }

    #[test]
    fn parses_mutating_exec_command_begin_as_coding_started() {
        let line = r#"{"type":"event_msg","payload":{"type":"exec_command_begin","call_id":"call_exec","command":["sed","-i","s/a/b/","src/App.svelte"]}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::CodingStarted {
                event_name: "exec_command_begin".to_string(),
                call_id: Some("call_exec".to_string()),
                tool_name: "exec_command".to_string(),
            }
        );
        assert_eq!(event.parsed_type(), Some("exec_command_begin"));
    }

    #[test]
    fn parses_exec_command_end_event() {
        let line =
            r#"{"type":"event_msg","payload":{"type":"exec_command_end","call_id":"call_exec"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::ToolCallCompleted {
                event_name: "exec_command_end".to_string(),
                call_id: Some("call_exec".to_string()),
            }
        );
        assert_eq!(event.parsed_type(), Some("exec_command_end"));
    }

    #[test]
    fn parses_web_search_begin_event() {
        let line =
            r#"{"type":"event_msg","payload":{"type":"web_search_begin","call_id":"call_search"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::ToolCallStarted {
                event_name: "web_search_begin".to_string(),
                call_id: Some("call_search".to_string()),
                tool_name: "web_search".to_string(),
            }
        );
        assert_eq!(event.parsed_type(), Some("web_search_begin"));
    }

    #[test]
    fn parses_web_search_end_event() {
        let line =
            r#"{"type":"event_msg","payload":{"type":"web_search_end","call_id":"call_search"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::ToolCallCompleted {
                event_name: "web_search_end".to_string(),
                call_id: Some("call_search".to_string()),
            }
        );
        assert_eq!(event.parsed_type(), Some("web_search_end"));
    }

    #[test]
    fn parses_function_call_output_from_response_item_payload() {
        let line = r#"{"type":"response_item","payload":{"type":"function_call_output","call_id":"call_123","output":"ok"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::ToolCallCompleted {
                event_name: "function_call_output".to_string(),
                call_id: Some("call_123".to_string()),
            }
        );
        assert_eq!(event.parsed_type(), Some("function_call_output"));
    }

    #[test]
    fn parses_mutating_function_call_as_coding_started() {
        let line = r#"{"type":"response_item","payload":{"type":"function_call","name":"exec_command","arguments":"{\"command\":[\"touch\",\"src/new-file.ts\"]}","call_id":"call_123"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::CodingStarted {
                event_name: "function_call".to_string(),
                call_id: Some("call_123".to_string()),
                tool_name: "exec_command".to_string(),
            }
        );
        assert_eq!(event.parsed_type(), Some("function_call"));
    }

    #[test]
    fn parses_mutating_local_shell_call_as_coding_started() {
        let line = r#"{"type":"response_item","payload":{"type":"local_shell_call","call_id":"call_shell","status":"in_progress","action":{"type":"exec","command":["mkdir","-p","src/generated"]}}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::CodingStarted {
                event_name: "local_shell_call".to_string(),
                call_id: Some("call_shell".to_string()),
                tool_name: "local_shell_call".to_string(),
            }
        );
        assert_eq!(event.parsed_type(), Some("local_shell_call"));
    }

    #[test]
    fn parses_turn_aborted_event() {
        let line =
            r#"{"type":"event_msg","payload":{"type":"turn_aborted","reason":"user_interrupt"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::TurnAborted {
                event_name: "turn_aborted".to_string(),
            }
        );
        assert_eq!(event.parsed_type(), Some("turn_aborted"));
    }

    #[test]
    fn parses_error_event_with_message() {
        let line =
            r#"{"type":"event_msg","payload":{"type":"error","message":"permission denied"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::Error {
                event_name: "error".to_string(),
                message: "permission denied".to_string(),
            }
        );
        assert_eq!(event.parsed_type(), Some("error"));
    }

    #[test]
    fn preserves_unknown_event_name_for_observability() {
        let line = r#"{"type":"response_item","payload":{"type":"something_weird"}}"#;
        let event = parse_session_line(line);

        assert_eq!(
            event,
            SessionEvent::Unknown {
                event_name: Some("something_weird".to_string()),
            }
        );
        assert_eq!(event.parsed_type(), Some("something_weird"));
        assert!(!event.parse_ok());
    }

    #[test]
    fn known_event_exposes_parsed_type_and_parse_ok() {
        let line =
            r#"{"type":"event_msg","payload":{"type":"exec_approval_request","call_id":"abc"}}"#;
        let event = parse_session_line(line);

        assert_eq!(event.parsed_type(), Some("exec_approval_request"));
        assert!(event.parse_ok());
    }

    #[test]
    fn invalid_json_exposes_no_parsed_type_and_parse_not_ok() {
        let event = parse_session_line("{");

        assert_eq!(event.parsed_type(), None);
        assert!(!event.parse_ok());
    }
}
