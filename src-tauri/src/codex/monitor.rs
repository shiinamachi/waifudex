#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorSnapshot {
    pub process_hint: &'static str,
    pub strategy: &'static str,
}

pub fn placeholder_snapshot() -> MonitorSnapshot {
    MonitorSnapshot {
        process_hint: "codex",
        strategy: "CLI wrapper integration will replace this placeholder snapshot.",
    }
}
