pub mod monitor;
pub mod parser;

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

pub const CODEX_STATUS_EVENT: &str = "waifudex://codex-status";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusKind {
    Idle,
    Thinking,
    Writing,
    RunningTests,
    Success,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusPayload {
    pub status: StatusKind,
    pub summary: String,
    pub detail: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    pub source: String,
}

impl StatusPayload {
    pub fn new(status: StatusKind, source: impl Into<String>) -> Self {
        let (summary, detail) = describe_status(status);

        Self {
            status,
            summary: summary.to_string(),
            detail: detail.to_string(),
            updated_at: timestamp_for(status),
            source: source.into(),
        }
    }
}

pub fn demo_sequence() -> Vec<StatusPayload> {
    [
        StatusKind::Idle,
        StatusKind::Thinking,
        StatusKind::Writing,
        StatusKind::RunningTests,
        StatusKind::Success,
        StatusKind::Error,
    ]
    .into_iter()
    .map(|status| StatusPayload::new(status, "demo"))
    .collect()
}

pub fn start_demo_emitter(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let sequence = demo_sequence();
        if sequence.is_empty() {
            return;
        }

        let mut ticker = tokio::time::interval(Duration::from_secs(3));
        let mut index = 0usize;

        loop {
            let payload = sequence[index % sequence.len()].clone();
            let _ = app.emit(CODEX_STATUS_EVENT, payload);
            index += 1;
            ticker.tick().await;
        }
    });
}

fn describe_status(status: StatusKind) -> (&'static str, &'static str) {
    match status {
        StatusKind::Idle => (
            "Waiting for the next Codex task",
            "The backend is connected and the mascot remains in a calm default posture.",
        ),
        StatusKind::Thinking => (
            "Thinking through the next change",
            "Codex is reviewing context and planning the next edit.",
        ),
        StatusKind::Writing => (
            "Writing implementation details",
            "Codex is actively mutating project files.",
        ),
        StatusKind::RunningTests => (
            "Running tests and build checks",
            "Codex is validating changes before it reports completion.",
        ),
        StatusKind::Success => (
            "Latest task completed cleanly",
            "The current step passed validation and can advance.",
        ),
        StatusKind::Error => (
            "A blocking issue needs attention",
            "The mascot should switch to an error expression until the next retry.",
        ),
    }
}

fn timestamp_for(status: StatusKind) -> String {
    match status {
        StatusKind::Idle => "2026-03-17T06:50:00.000Z",
        StatusKind::Thinking => "2026-03-17T06:50:15.000Z",
        StatusKind::Writing => "2026-03-17T06:50:30.000Z",
        StatusKind::RunningTests => "2026-03-17T06:50:45.000Z",
        StatusKind::Success => "2026-03-17T06:51:00.000Z",
        StatusKind::Error => "2026-03-17T06:51:15.000Z",
    }
    .to_string()
}
