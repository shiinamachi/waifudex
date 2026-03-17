pub mod discovery;
pub mod liveness;
pub mod monitor;
pub mod parser;
pub mod reducer;
pub mod session_reader;

use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::contracts::runtime::RuntimeSnapshot;
pub use crate::contracts::runtime::RuntimeStatus as StatusKind;
pub use monitor::start_monitor;
pub use parser::SessionEvent;

pub fn snapshot_for_status(
    status: StatusKind,
    source: impl Into<String>,
    session_id: Option<String>,
) -> RuntimeSnapshot {
    let (summary, detail) = describe_status(status);
    RuntimeSnapshot {
        session_id,
        status,
        summary: summary.to_string(),
        detail: detail.to_string(),
        updated_at: timestamp_now(),
        source: source.into(),
        revision: 0,
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

pub fn timestamp_now() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}
