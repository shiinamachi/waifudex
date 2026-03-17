use super::{liveness::LivenessSnapshot, parser::SessionEvent, StatusKind, StatusPayload};

#[derive(Debug, Clone)]
pub struct StatusReducer {
    source: String,
    in_task: bool,
    active_tool_status: Option<StatusKind>,
}

impl StatusReducer {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            in_task: false,
            active_tool_status: None,
        }
    }

    pub fn reduce(
        &mut self,
        event: Option<&SessionEvent>,
        liveness: LivenessSnapshot,
    ) -> StatusPayload {
        let status = match event {
            Some(SessionEvent::TaskStarted) => {
                self.in_task = true;
                self.active_tool_status = None;
                StatusKind::Thinking
            }
            Some(SessionEvent::TaskCompleted) => {
                self.in_task = false;
                self.active_tool_status = None;
                StatusKind::Success
            }
            Some(SessionEvent::ToolCallStarted { tool_name }) => {
                let status = classify_tool_status(tool_name);
                self.in_task = true;
                self.active_tool_status = Some(status);
                status
            }
            Some(SessionEvent::ToolCallCompleted) => {
                self.active_tool_status = None;
                if self.in_task {
                    StatusKind::Thinking
                } else {
                    StatusKind::Idle
                }
            }
            Some(SessionEvent::MessageDelta | SessionEvent::TokenCount) => {
                active_or_idle(self.in_task, self.active_tool_status)
            }
            Some(SessionEvent::Error { .. }) => {
                self.in_task = false;
                self.active_tool_status = None;
                StatusKind::Error
            }
            Some(SessionEvent::Unknown) | None => {
                fallback_status(liveness, self.in_task, self.active_tool_status)
            }
        };

        StatusPayload::new(status, self.source.clone())
    }
}

fn classify_tool_status(tool_name: &str) -> StatusKind {
    let normalized = tool_name.to_ascii_lowercase();
    if ["test", "build", "vitest", "cargo test", "pnpm test"]
        .iter()
        .any(|keyword| normalized.contains(keyword))
    {
        return StatusKind::RunningTests;
    }

    StatusKind::Writing
}

fn fallback_status(
    liveness: LivenessSnapshot,
    in_task: bool,
    active_tool_status: Option<StatusKind>,
) -> StatusKind {
    if !liveness.has_live_process {
        return StatusKind::Idle;
    }

    if in_task {
        return active_tool_status.unwrap_or(StatusKind::Thinking);
    }

    StatusKind::Idle
}

fn active_or_idle(in_task: bool, active_tool_status: Option<StatusKind>) -> StatusKind {
    if in_task {
        active_tool_status.unwrap_or(StatusKind::Thinking)
    } else {
        StatusKind::Idle
    }
}

#[cfg(test)]
mod reducer_tests {
    use crate::codex::{
        liveness::LivenessSnapshot, parser::SessionEvent, reducer::StatusReducer, StatusKind,
    };

    #[test]
    fn task_started_reduces_to_thinking() {
        let mut reducer = StatusReducer::new("monitor");
        let payload = reducer.reduce(
            Some(&SessionEvent::TaskStarted),
            LivenessSnapshot::offline(),
        );

        assert_eq!(payload.status, StatusKind::Thinking);
    }

    #[test]
    fn test_like_tool_names_reduce_to_running_tests() {
        let mut reducer = StatusReducer::new("monitor");
        let payload = reducer.reduce(
            Some(&SessionEvent::ToolCallStarted {
                tool_name: "cargo test".to_string(),
            }),
            LivenessSnapshot::online(),
        );

        assert_eq!(payload.status, StatusKind::RunningTests);
    }

    #[test]
    fn write_like_tool_names_reduce_to_writing() {
        let mut reducer = StatusReducer::new("monitor");
        let payload = reducer.reduce(
            Some(&SessionEvent::ToolCallStarted {
                tool_name: "apply_patch".to_string(),
            }),
            LivenessSnapshot::online(),
        );

        assert_eq!(payload.status, StatusKind::Writing);
    }

    #[test]
    fn error_events_reduce_to_error() {
        let mut reducer = StatusReducer::new("monitor");
        let payload = reducer.reduce(
            Some(&SessionEvent::Error {
                message: "permission denied".to_string(),
            }),
            LivenessSnapshot::online(),
        );

        assert_eq!(payload.status, StatusKind::Error);
    }

    #[test]
    fn no_recent_events_and_no_live_process_reduce_to_idle() {
        let mut reducer = StatusReducer::new("monitor");
        let payload = reducer.reduce(None, LivenessSnapshot::offline());

        assert_eq!(payload.status, StatusKind::Idle);
    }

    #[test]
    fn active_tool_status_is_retained_while_process_is_still_live() {
        let mut reducer = StatusReducer::new("monitor");
        let active = reducer.reduce(
            Some(&SessionEvent::ToolCallStarted {
                tool_name: "cargo test".to_string(),
            }),
            LivenessSnapshot::online(),
        );

        assert_eq!(active.status, StatusKind::RunningTests);

        let retained = reducer.reduce(None, LivenessSnapshot::online());
        assert_eq!(retained.status, StatusKind::RunningTests);
    }

    #[test]
    fn tool_call_completion_returns_to_thinking_while_task_is_active() {
        let mut reducer = StatusReducer::new("monitor");
        let _ = reducer.reduce(
            Some(&SessionEvent::ToolCallStarted {
                tool_name: "cargo test".to_string(),
            }),
            LivenessSnapshot::online(),
        );

        let payload = reducer.reduce(
            Some(&SessionEvent::ToolCallCompleted),
            LivenessSnapshot::online(),
        );
        assert_eq!(payload.status, StatusKind::Thinking);
    }

    #[test]
    fn completed_task_falls_back_to_idle_while_process_remains_open() {
        let mut reducer = StatusReducer::new("monitor");
        let _ = reducer.reduce(Some(&SessionEvent::TaskStarted), LivenessSnapshot::online());
        let done = reducer.reduce(
            Some(&SessionEvent::TaskCompleted),
            LivenessSnapshot::online(),
        );
        assert_eq!(done.status, StatusKind::Success);

        let idle = reducer.reduce(None, LivenessSnapshot::online());
        assert_eq!(idle.status, StatusKind::Idle);
    }

    #[test]
    fn active_task_falls_back_to_idle_when_process_goes_offline() {
        let mut reducer = StatusReducer::new("monitor");
        let _ = reducer.reduce(Some(&SessionEvent::TaskStarted), LivenessSnapshot::online());

        let idle = reducer.reduce(None, LivenessSnapshot::offline());
        assert_eq!(idle.status, StatusKind::Idle);
    }
}
