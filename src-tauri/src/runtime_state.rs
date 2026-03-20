use std::sync::Mutex;

use tauri::State;

use crate::contracts::runtime::{RuntimeBootstrap, RuntimeSnapshot};

#[derive(Debug, Default)]
pub struct RuntimeState {
    inner: Mutex<RuntimeStateInner>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineEventCursor {
    pub sequence: u64,
    pub event_id: String,
}

#[derive(Debug, Default)]
struct RuntimeStateInner {
    latest_snapshot: Option<RuntimeSnapshot>,
    next_revision: u64,
    sequence_session_id: Option<String>,
    next_sequence: u64,
    next_event_ordinal: u64,
}

impl RuntimeState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bootstrap(&self) -> RuntimeBootstrap {
        let inner = self.inner.lock().expect("runtime state mutex poisoned");
        RuntimeBootstrap::from_snapshot(inner.latest_snapshot.clone())
    }

    pub fn record_snapshot(&self, snapshot: RuntimeSnapshot) -> RuntimeSnapshot {
        let mut inner = self.inner.lock().expect("runtime state mutex poisoned");
        inner.record_snapshot(snapshot)
    }

    pub fn next_timeline_event(&self, session_id: Option<&str>) -> TimelineEventCursor {
        let mut inner = self.inner.lock().expect("runtime state mutex poisoned");
        let sequence = inner.next_sequence_for(session_id);
        let ordinal = inner.next_event_ordinal();
        let session_label = session_id.unwrap_or("none");
        TimelineEventCursor {
            sequence,
            event_id: format!("{session_label}:{sequence}:{ordinal}"),
        }
    }
}

impl RuntimeStateInner {
    fn record_snapshot(&mut self, mut snapshot: RuntimeSnapshot) -> RuntimeSnapshot {
        snapshot.revision = self.next_revision;
        self.next_revision = self.next_revision.saturating_add(1);
        self.latest_snapshot = Some(snapshot.clone());
        snapshot
    }

    fn next_sequence_for(&mut self, session_id: Option<&str>) -> u64 {
        let normalized = session_id.map(ToString::to_string);
        if self.sequence_session_id != normalized {
            self.sequence_session_id = normalized;
            self.next_sequence = 1;
            return 0;
        }

        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        sequence
    }

    fn next_event_ordinal(&mut self) -> u64 {
        let ordinal = self.next_event_ordinal;
        self.next_event_ordinal = self.next_event_ordinal.saturating_add(1);
        ordinal
    }
}

#[tauri::command]
pub fn get_runtime_bootstrap(state: State<'_, RuntimeState>) -> RuntimeBootstrap {
    state.bootstrap()
}
