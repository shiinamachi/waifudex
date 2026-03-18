use crate::codex::backend::command_runner::{CommandRunner, ProcessWslCommandRunner};
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

#[derive(Debug)]
pub struct LivenessProbe<R: CommandRunner = ProcessWslCommandRunner> {
    process_hint: String,
    wsl_runner: R,
}

impl LivenessProbe<ProcessWslCommandRunner> {
    pub fn new(process_hint: impl Into<String>) -> Self {
        Self {
            process_hint: process_hint.into(),
            wsl_runner: ProcessWslCommandRunner,
        }
    }
}

impl<R: CommandRunner> LivenessProbe<R> {
    #[cfg(test)]
    pub fn with_runner(process_hint: impl Into<String>, wsl_runner: R) -> Self {
        Self {
            process_hint: process_hint.into(),
            wsl_runner,
        }
    }

    pub fn snapshot(
        &mut self,
        backend_kind: &str,
        sessions_root_display: &str,
    ) -> LivenessSnapshot {
        if backend_kind == "wsl_command" {
            if let Some(distro) = wsl_distro_from_sessions_root_display(sessions_root_display) {
                if let Some(snapshot) = self.snapshot_wsl(distro) {
                    return snapshot;
                }
            }
        }

        self.snapshot_local()
    }

    fn snapshot_local(&self) -> LivenessSnapshot {
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

    fn snapshot_wsl(&mut self, distro: &str) -> Option<LivenessSnapshot> {
        let process_hint = shell_quote(&self.process_hint);
        let command = format!(
            "ps -eo comm=,args= | awk 'BEGIN {{ target = tolower(target) }} index(tolower($0), target) {{ found = 1 }} END {{ if (found) print \"live\"; else print \"idle\"; }}' target={process_hint}"
        );
        let output = self
            .wsl_runner
            .run(&["-d", distro, "--", "sh", "-lc", &command])
            .ok()?;
        if !output.success {
            return None;
        }

        Some(LivenessSnapshot {
            has_live_process: output.stdout.trim() == "live",
        })
    }
}

fn wsl_distro_from_sessions_root_display(sessions_root_display: &str) -> Option<&str> {
    let (distro, path) = sessions_root_display.split_once(':')?;
    if distro.is_empty() || path.is_empty() {
        return None;
    }

    Some(distro)
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'\''"#))
}

#[cfg(test)]
mod liveness_tests {
    use std::{collections::VecDeque, io};

    use crate::codex::backend::command_runner::{CommandOutput, CommandRunner};

    use super::LivenessProbe;

    #[derive(Default)]
    struct FakeRunner {
        outputs: VecDeque<io::Result<CommandOutput>>,
    }

    impl FakeRunner {
        fn push_ok(&mut self, stdout: &str) {
            self.outputs.push_back(Ok(CommandOutput {
                success: true,
                stdout: stdout.to_string(),
                stderr: String::new(),
            }));
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(&mut self, _args: &[&str]) -> io::Result<CommandOutput> {
            self.outputs
                .pop_front()
                .unwrap_or_else(|| Err(io::Error::other("missing fake output")))
        }
    }

    #[test]
    fn treats_wsl_backend_as_online_when_the_distro_has_a_live_codex_process() {
        let mut runner = FakeRunner::default();
        runner.push_ok("live\n");

        let mut probe = LivenessProbe::with_runner("codex", runner);
        let snapshot = probe.snapshot("wsl_command", "Ubuntu:/home/tester/.codex/sessions");

        assert!(snapshot.has_live_process);
    }
}
