pub mod backend;
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
    sessions_root: impl Into<String>,
) -> RuntimeSnapshot {
    let (summary, detail) = describe_status(status);
    RuntimeSnapshot {
        session_id,
        status,
        summary: summary.to_string(),
        detail: detail.to_string(),
        sessions_root: sessions_root.into(),
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
        StatusKind::CodexNotInstalled => (
            "Codex sessions root not found",
            "Waifudex could not find the configured Codex sessions directory, so it cannot observe Codex runtime state.",
        ),
        StatusKind::Thinking => (
            "Thinking or using tools",
            "Codex is reasoning through the task or using non-coding tools.",
        ),
        StatusKind::Coding => (
            "Writing or patching code",
            "Codex is editing code or applying a patch to the workspace.",
        ),
        StatusKind::Question => (
            "Waiting for your input or approval",
            "Codex needs an answer, approval, or permission update before it can continue.",
        ),
        StatusKind::Complete => (
            "Current task completed",
            "Codex finished the current task and will fall back to idle if no new work arrives.",
        ),
    }
}

pub fn timestamp_now() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}
