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
