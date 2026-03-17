pub mod discovery;
pub mod liveness;
pub mod monitor;
pub mod parser;
pub mod reducer;
pub mod session_reader;

use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub use monitor::start_monitor;
pub use parser::SessionEvent;

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
    let _ = status;
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}
