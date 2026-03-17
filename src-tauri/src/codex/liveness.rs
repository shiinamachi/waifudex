use sysinfo::{ProcessesToUpdate, System};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LivenessSnapshot {
    pub has_live_process: bool,
}

impl LivenessSnapshot {
    pub fn online() -> Self {
        Self {
            has_live_process: true,
        }
    }

    pub fn offline() -> Self {
        Self {
            has_live_process: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LivenessProbe {
    process_hint: String,
}

impl LivenessProbe {
    pub fn new(process_hint: impl Into<String>) -> Self {
        Self {
            process_hint: process_hint.into(),
        }
    }

    pub fn snapshot(&self) -> LivenessSnapshot {
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::All, false);

        let process_hint = self.process_hint.to_ascii_lowercase();
        let has_live_process = system.processes().values().any(|process| {
            process
                .name()
                .to_string_lossy()
                .to_ascii_lowercase()
                .contains(&process_hint)
        });

        LivenessSnapshot { has_live_process }
    }
}
