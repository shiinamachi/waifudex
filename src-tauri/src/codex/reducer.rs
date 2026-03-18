use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use crate::contracts::runtime::RuntimeSnapshot;

use super::{
    liveness::LivenessSnapshot,
    parser::{QuestionKind, SessionEvent},
    snapshot_for_status, StatusKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActivityKind {
    Thinking,
    Coding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingQuestion {
    call_id: Option<String>,
    kind: QuestionKind,
}

#[derive(Debug, Clone)]
pub struct StatusReducer {
    source: String,
    turn_active: bool,
    question_pending: Option<PendingQuestion>,
    active_calls: HashMap<String, ActivityKind>,
    active_thinking_ops: usize,
    active_coding_ops: usize,
    complete_until: Option<Instant>,
    last_event_at: Option<Instant>,
    aborted: bool,
}

impl StatusReducer {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            turn_active: false,
            question_pending: None,
            active_calls: HashMap::new(),
            active_thinking_ops: 0,
            active_coding_ops: 0,
            complete_until: None,
            last_event_at: None,
            aborted: false,
        }
    }

    pub fn reduce(
        &mut self,
        event: Option<&SessionEvent>,
        session_id: Option<String>,
        sessions_root: &str,
        liveness: LivenessSnapshot,
    ) -> RuntimeSnapshot {
        self.reduce_at(event, session_id, sessions_root, liveness, Instant::now())
    }

    pub fn reduce_at(
        &mut self,
        event: Option<&SessionEvent>,
        session_id: Option<String>,
        sessions_root: &str,
        liveness: LivenessSnapshot,
        now: Instant,
    ) -> RuntimeSnapshot {
        match event {
            Some(SessionEvent::TaskStarted { .. }) => {
                self.aborted = false;
                self.turn_active = true;
                self.question_pending = None;
                self.complete_until = None;
            }
            Some(SessionEvent::TurnAborted { .. }) => {
                self.turn_active = false;
                self.question_pending = None;
                self.active_calls.clear();
                self.active_thinking_ops = 0;
                self.active_coding_ops = 0;
                self.complete_until = None;
                self.aborted = true;
            }
            _ if self.aborted => {}
            Some(SessionEvent::TaskCompleted { .. }) => {
                self.turn_active = false;
                self.question_pending = None;
                self.active_calls.clear();
                self.active_thinking_ops = 0;
                self.active_coding_ops = 0;
                self.complete_until = Some(now + COMPLETE_GRACE_PERIOD);
            }
            Some(SessionEvent::QuestionAsked { call_id, kind, .. }) => {
                self.complete_until = None;
                self.question_pending = Some(PendingQuestion {
                    call_id: call_id.clone(),
                    kind: *kind,
                });
            }
            Some(SessionEvent::CodingStarted { call_id, .. }) => {
                self.turn_active = true;
                self.complete_until = None;
                self.clear_question_on_progress();
                self.start_activity(call_id.as_deref(), ActivityKind::Coding);
            }
            Some(SessionEvent::CodingCompleted { call_id, .. }) => {
                self.finish_activity(call_id.as_deref(), Some(ActivityKind::Coding));
            }
            Some(SessionEvent::ToolCallStarted { call_id, .. }) => {
                self.turn_active = true;
                self.complete_until = None;
                self.clear_question_on_progress();
                self.start_activity(call_id.as_deref(), ActivityKind::Thinking);
            }
            Some(SessionEvent::ToolCallCompleted { call_id, .. }) => {
                self.finish_activity(call_id.as_deref(), None);
            }
            Some(SessionEvent::MessageDelta { .. }) | Some(SessionEvent::TokenCount { .. }) => {
                self.turn_active = true;
                self.complete_until = None;
            }
            Some(SessionEvent::Error { .. }) | Some(SessionEvent::Unknown { .. }) | None => {}
        }

        match event {
            Some(SessionEvent::Unknown { .. }) | None => {}
            Some(_) => {
                self.last_event_at = Some(now);
            }
        }

        let status = match event {
            Some(SessionEvent::Unknown { .. }) | None => self.current_status(liveness, now, true),
            Some(_) => self.current_status(liveness, now, false),
        };
        snapshot_for_status(status, self.source.clone(), session_id, sessions_root)
    }

    fn start_activity(&mut self, call_id: Option<&str>, kind: ActivityKind) {
        if let Some(call_id) = call_id {
            match self.active_calls.get(call_id).copied() {
                Some(existing) if existing == kind => return,
                Some(existing) => {
                    self.decrement_activity(existing);
                    self.active_calls.insert(call_id.to_string(), kind);
                    self.increment_activity(kind);
                    return;
                }
                None => {
                    self.active_calls.insert(call_id.to_string(), kind);
                    self.increment_activity(kind);
                    return;
                }
            }
        }

        self.increment_activity(kind);
    }

    fn finish_activity(&mut self, call_id: Option<&str>, fallback_kind: Option<ActivityKind>) {
        if let Some(call_id) = call_id {
            if let Some(kind) = self.active_calls.remove(call_id) {
                self.decrement_activity(kind);
                return;
            }
        }

        if let Some(kind) = fallback_kind {
            self.decrement_activity(kind);
        } else if self.active_thinking_ops > 0 {
            self.active_thinking_ops -= 1;
        } else if self.active_coding_ops > 0 {
            self.active_coding_ops -= 1;
        }
    }

    fn clear_question_on_progress(&mut self) {
        if self.question_pending.is_some() {
            self.question_pending = None;
        }
    }

    fn increment_activity(&mut self, kind: ActivityKind) {
        match kind {
            ActivityKind::Thinking => {
                self.active_thinking_ops = self.active_thinking_ops.saturating_add(1)
            }
            ActivityKind::Coding => {
                self.active_coding_ops = self.active_coding_ops.saturating_add(1)
            }
        }
    }

    fn decrement_activity(&mut self, kind: ActivityKind) {
        match kind {
            ActivityKind::Thinking => {
                self.active_thinking_ops = self.active_thinking_ops.saturating_sub(1)
            }
            ActivityKind::Coding => {
                self.active_coding_ops = self.active_coding_ops.saturating_sub(1)
            }
        }
    }

    fn current_status(
        &self,
        liveness: LivenessSnapshot,
        now: Instant,
        apply_liveness_gate: bool,
    ) -> StatusKind {
        if self.question_pending.is_some() {
            return StatusKind::Question;
        }
        if self.complete_until.is_some_and(|deadline| now < deadline) {
            return StatusKind::Complete;
        }
        if apply_liveness_gate && !liveness.has_live_process {
            let recently_active = self
                .last_event_at
                .is_some_and(|t| now.duration_since(t) < EVENT_LIVENESS_GRACE);
            if !recently_active {
                return StatusKind::Idle;
            }
        }
        if self.active_coding_ops > 0 {
            return StatusKind::Coding;
        }
        if self.turn_active || self.active_thinking_ops > 0 {
            return StatusKind::Thinking;
        }

        StatusKind::Idle
    }
}

const COMPLETE_GRACE_PERIOD: Duration = Duration::from_secs(10);
const EVENT_LIVENESS_GRACE: Duration = Duration::from_secs(30);

#[cfg(test)]
mod reducer_tests {
    use std::time::{Duration, Instant};

    use crate::codex::{
        liveness::LivenessSnapshot,
        parser::{QuestionKind, SessionEvent},
        reducer::StatusReducer,
        StatusKind,
    };

    #[test]
    fn task_started_reduces_to_thinking() {
        let mut reducer = StatusReducer::new("monitor");
        let payload = reducer.reduce(
            Some(&SessionEvent::TaskStarted {
                event_name: "task_started".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::offline(),
        );

        assert_eq!(payload.status, StatusKind::Thinking);
    }

    #[test]
    fn task_started_clears_pending_question() {
        let mut reducer = StatusReducer::new("monitor");
        let _ = reducer.reduce(
            Some(&SessionEvent::QuestionAsked {
                event_name: "request_user_input".to_string(),
                call_id: Some("question-call".to_string()),
                kind: QuestionKind::RequestUserInput,
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        let payload = reducer.reduce(
            Some(&SessionEvent::TaskStarted {
                event_name: "task_started".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        assert_eq!(payload.status, StatusKind::Thinking);
    }

    #[test]
    fn question_events_reduce_to_question() {
        let mut reducer = StatusReducer::new("monitor");
        let payload = reducer.reduce(
            Some(&SessionEvent::QuestionAsked {
                event_name: "request_user_input".to_string(),
                call_id: Some("call-1".to_string()),
                kind: QuestionKind::RequestUserInput,
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        assert_eq!(payload.status, StatusKind::Question);
    }

    #[test]
    fn coding_events_reduce_to_coding() {
        let mut reducer = StatusReducer::new("monitor");
        let payload = reducer.reduce(
            Some(&SessionEvent::CodingStarted {
                event_name: "patch_apply_begin".to_string(),
                call_id: Some("call-patch".to_string()),
                tool_name: "apply_patch".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        assert_eq!(payload.status, StatusKind::Coding);
    }

    #[test]
    fn message_delta_without_prior_activity_enters_thinking() {
        let mut reducer = StatusReducer::new("monitor");
        let payload = reducer.reduce(
            Some(&SessionEvent::MessageDelta {
                event_name: "agent_message".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::offline(),
        );

        assert_eq!(payload.status, StatusKind::Thinking);
    }

    #[test]
    fn token_count_without_prior_activity_enters_thinking() {
        let mut reducer = StatusReducer::new("monitor");
        let payload = reducer.reduce(
            Some(&SessionEvent::TokenCount {
                event_name: "token_count".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::offline(),
        );

        assert_eq!(payload.status, StatusKind::Thinking);
    }

    #[test]
    fn message_delta_does_not_clear_pending_question() {
        let mut reducer = StatusReducer::new("monitor");
        let _ = reducer.reduce(
            Some(&SessionEvent::QuestionAsked {
                event_name: "request_user_input".to_string(),
                call_id: Some("question-call".to_string()),
                kind: QuestionKind::RequestUserInput,
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        let payload = reducer.reduce(
            Some(&SessionEvent::MessageDelta {
                event_name: "agent_message".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        assert_eq!(payload.status, StatusKind::Question);
    }

    #[test]
    fn token_count_does_not_clear_pending_question() {
        let mut reducer = StatusReducer::new("monitor");
        let _ = reducer.reduce(
            Some(&SessionEvent::QuestionAsked {
                event_name: "request_user_input".to_string(),
                call_id: Some("question-call".to_string()),
                kind: QuestionKind::RequestUserInput,
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        let payload = reducer.reduce(
            Some(&SessionEvent::TokenCount {
                event_name: "token_count".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        assert_eq!(payload.status, StatusKind::Question);
    }

    #[test]
    fn user_message_without_prior_activity_enters_thinking() {
        let mut reducer = StatusReducer::new("monitor");
        let payload = reducer.reduce(
            Some(&SessionEvent::MessageDelta {
                event_name: "user_message".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::offline(),
        );

        assert_eq!(payload.status, StatusKind::Thinking);
    }

    #[test]
    fn duplicate_call_start_with_same_call_id_does_not_double_count() {
        let mut reducer = StatusReducer::new("monitor");
        let _ = reducer.reduce(
            Some(&SessionEvent::ToolCallStarted {
                event_name: "function_call".to_string(),
                call_id: Some("call-1".to_string()),
                tool_name: "exec_command".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        let _ = reducer.reduce(
            Some(&SessionEvent::ToolCallStarted {
                event_name: "exec_command_begin".to_string(),
                call_id: Some("call-1".to_string()),
                tool_name: "exec_command".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        let payload = reducer.reduce(
            Some(&SessionEvent::ToolCallCompleted {
                event_name: "exec_command_end".to_string(),
                call_id: Some("call-1".to_string()),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        assert_eq!(payload.status, StatusKind::Thinking);
    }

    #[test]
    fn no_recent_events_and_no_live_process_reduce_to_idle() {
        let mut reducer = StatusReducer::new("monitor");
        let payload = reducer.reduce(
            None,
            None,
            "/home/tester/.codex/sessions",
            LivenessSnapshot::offline(),
        );

        assert_eq!(payload.status, StatusKind::Idle);
    }

    #[test]
    fn active_thinking_state_is_retained_while_process_is_still_live() {
        let mut reducer = StatusReducer::new("monitor");
        let active = reducer.reduce(
            Some(&SessionEvent::ToolCallStarted {
                event_name: "function_call".to_string(),
                call_id: Some("call-1".to_string()),
                tool_name: "exec_command".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        assert_eq!(active.status, StatusKind::Thinking);

        let retained = reducer.reduce(
            None,
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );
        assert_eq!(retained.status, StatusKind::Thinking);
    }

    #[test]
    fn question_has_priority_over_coding_and_thinking() {
        let mut reducer = StatusReducer::new("monitor");
        let _ = reducer.reduce(
            Some(&SessionEvent::CodingStarted {
                event_name: "patch_apply_begin".to_string(),
                call_id: Some("call-patch".to_string()),
                tool_name: "apply_patch".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        let payload = reducer.reduce(
            Some(&SessionEvent::QuestionAsked {
                event_name: "exec_approval_request".to_string(),
                call_id: Some("call-approval".to_string()),
                kind: QuestionKind::ExecApproval,
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        assert_eq!(payload.status, StatusKind::Question);
    }

    #[test]
    fn any_progress_event_clears_question_even_with_different_call_id() {
        let mut reducer = StatusReducer::new("monitor");
        let _ = reducer.reduce(
            Some(&SessionEvent::QuestionAsked {
                event_name: "request_user_input".to_string(),
                call_id: Some("question-call".to_string()),
                kind: QuestionKind::RequestUserInput,
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        let payload = reducer.reduce(
            Some(&SessionEvent::ToolCallStarted {
                event_name: "function_call".to_string(),
                call_id: Some("different-progress-call".to_string()),
                tool_name: "exec_command".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        assert_eq!(payload.status, StatusKind::Thinking);
    }

    #[test]
    fn tool_completion_alone_does_not_clear_pending_question() {
        let mut reducer = StatusReducer::new("monitor");
        let _ = reducer.reduce(
            Some(&SessionEvent::QuestionAsked {
                event_name: "request_user_input".to_string(),
                call_id: Some("question-call".to_string()),
                kind: QuestionKind::RequestUserInput,
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        let payload = reducer.reduce(
            Some(&SessionEvent::ToolCallCompleted {
                event_name: "function_call_output".to_string(),
                call_id: Some("different-progress-call".to_string()),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        assert_eq!(payload.status, StatusKind::Question);
    }

    #[test]
    fn coding_completion_alone_does_not_clear_pending_question() {
        let mut reducer = StatusReducer::new("monitor");
        let _ = reducer.reduce(
            Some(&SessionEvent::QuestionAsked {
                event_name: "request_user_input".to_string(),
                call_id: Some("question-call".to_string()),
                kind: QuestionKind::RequestUserInput,
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        let payload = reducer.reduce(
            Some(&SessionEvent::CodingCompleted {
                event_name: "patch_apply_end".to_string(),
                call_id: Some("different-progress-call".to_string()),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        assert_eq!(payload.status, StatusKind::Question);
    }

    #[test]
    fn task_complete_reduces_to_complete() {
        let mut reducer = StatusReducer::new("monitor");
        let payload = reducer.reduce(
            Some(&SessionEvent::TaskCompleted {
                event_name: "task_complete".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );
        assert_eq!(payload.status, StatusKind::Complete);
    }

    #[test]
    fn new_activity_clears_complete_state() {
        let mut reducer = StatusReducer::new("monitor");
        let _ = reducer.reduce(
            Some(&SessionEvent::TaskCompleted {
                event_name: "task_complete".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        let thinking = reducer.reduce(
            Some(&SessionEvent::TaskStarted {
                event_name: "task_started".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );
        assert_eq!(thinking.status, StatusKind::Thinking);
    }

    #[test]
    fn turn_aborted_clears_active_state_and_returns_to_idle() {
        let mut reducer = StatusReducer::new("monitor");
        let _ = reducer.reduce(
            Some(&SessionEvent::CodingStarted {
                event_name: "patch_apply_begin".to_string(),
                call_id: Some("call-patch".to_string()),
                tool_name: "apply_patch".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        let aborted = reducer.reduce(
            Some(&SessionEvent::TurnAborted {
                event_name: "turn_aborted".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        assert_eq!(aborted.status, StatusKind::Idle);
    }

    #[test]
    fn recent_event_prevents_liveness_gate_from_forcing_idle() {
        let mut reducer = StatusReducer::new("monitor");
        let now = Instant::now();

        let _ = reducer.reduce_at(
            Some(&SessionEvent::TaskStarted {
                event_name: "task_started".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
            now,
        );

        let payload = reducer.reduce_at(
            None,
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::offline(),
            now + Duration::from_millis(250),
        );

        assert_eq!(payload.status, StatusKind::Thinking);
    }

    #[test]
    fn stale_event_allows_liveness_gate_to_force_idle() {
        let mut reducer = StatusReducer::new("monitor");
        let now = Instant::now();

        let _ = reducer.reduce_at(
            Some(&SessionEvent::TaskStarted {
                event_name: "task_started".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
            now,
        );

        let payload = reducer.reduce_at(
            None,
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::offline(),
            now + Duration::from_secs(31),
        );

        assert_eq!(payload.status, StatusKind::Idle);
    }

    #[test]
    fn trailing_message_after_abort_stays_idle() {
        let mut reducer = StatusReducer::new("monitor");

        let _ = reducer.reduce(
            Some(&SessionEvent::TaskStarted {
                event_name: "task_started".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        let _ = reducer.reduce(
            Some(&SessionEvent::TurnAborted {
                event_name: "turn_aborted".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        let payload = reducer.reduce(
            Some(&SessionEvent::MessageDelta {
                event_name: "agent_message".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        assert_eq!(payload.status, StatusKind::Idle);
    }

    #[test]
    fn trailing_tool_completion_after_abort_stays_idle() {
        let mut reducer = StatusReducer::new("monitor");

        let _ = reducer.reduce(
            Some(&SessionEvent::ToolCallStarted {
                event_name: "function_call".to_string(),
                call_id: Some("call-1".to_string()),
                tool_name: "exec_command".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        let _ = reducer.reduce(
            Some(&SessionEvent::TurnAborted {
                event_name: "turn_aborted".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        let payload = reducer.reduce(
            Some(&SessionEvent::ToolCallCompleted {
                event_name: "function_call_output".to_string(),
                call_id: Some("call-1".to_string()),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        assert_eq!(payload.status, StatusKind::Idle);
    }

    #[test]
    fn new_task_after_abort_resumes_normally() {
        let mut reducer = StatusReducer::new("monitor");

        let _ = reducer.reduce(
            Some(&SessionEvent::TurnAborted {
                event_name: "turn_aborted".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        let _ = reducer.reduce(
            Some(&SessionEvent::MessageDelta {
                event_name: "agent_message".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        let payload = reducer.reduce(
            Some(&SessionEvent::TaskStarted {
                event_name: "task_started".to_string(),
            }),
            Some("session-a".to_string()),
            "/home/tester/.codex/sessions",
            LivenessSnapshot::online(),
        );

        assert_eq!(payload.status, StatusKind::Thinking);
    }
}
