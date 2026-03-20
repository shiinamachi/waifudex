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
