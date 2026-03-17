use std::{
    collections::HashMap,
    io,
    path::PathBuf,
    time::{Duration, UNIX_EPOCH},
};

use super::{
    command_runner::{CommandOutput, CommandRunner, ProcessWslCommandRunner},
    SessionBackend, SessionCandidate,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct RootCandidate {
    distro: String,
    user_name: String,
    sessions_root: String,
}

pub struct WslCommandBackend<R: CommandRunner = ProcessWslCommandRunner> {
    runner: R,
    sessions_root_display: String,
    sessions_root_available: bool,
    distro: Option<String>,
    sessions_root: Option<String>,
    previous_sizes: HashMap<String, u64>,
    active_key: Option<String>,
    offset: u64,
    pending: String,
}

impl WslCommandBackend<ProcessWslCommandRunner> {
    pub fn discover(preferred_user: Option<String>) -> Self {
        Self::discover_with_runner(ProcessWslCommandRunner, preferred_user)
    }
}

impl<R: CommandRunner> WslCommandBackend<R> {
    pub fn discover_with_runner(mut runner: R, preferred_user: Option<String>) -> Self {
        let distros = match run_wsl(&mut runner, &["-l", "-q"]) {
            Ok(output) => {
                if !output.success {
                    return Self::unavailable(runner);
                }
                parse_lines(&output.stdout)
            }
            Err(_) => return Self::unavailable(runner),
        };

        let mut roots = Vec::new();
        for distro in distros {
            let home_output = match run_wsl(
                &mut runner,
                &["-d", &distro, "--", "sh", "-lc", "printf %s \"$HOME\""],
            ) {
                Ok(output) => output,
                Err(_) => continue,
            };
            if !home_output.success {
                continue;
            }

            let home = home_output.stdout.trim();
            if home.is_empty() {
                continue;
            }

            let sessions_root = format!("{home}/.codex/sessions");
            let exists_command = format!(
                "[ -d {} ] && printf ok || printf missing",
                shell_quote(&sessions_root)
            );
            let exists_output = match run_wsl(
                &mut runner,
                &["-d", &distro, "--", "sh", "-lc", &exists_command],
            ) {
                Ok(output) => output,
                Err(_) => continue,
            };

            if exists_output.success && exists_output.stdout.trim() == "ok" {
                let user_name = home.rsplit('/').next().unwrap_or(home).to_string();
                roots.push(RootCandidate {
                    distro,
                    user_name,
                    sessions_root,
                });
            }
        }

        roots.sort_by(|left, right| {
            let left_preferred = preferred_user.as_deref() == Some(left.user_name.as_str());
            let right_preferred = preferred_user.as_deref() == Some(right.user_name.as_str());

            right_preferred
                .cmp(&left_preferred)
                .then_with(|| left.distro.cmp(&right.distro))
                .then_with(|| left.user_name.cmp(&right.user_name))
        });

        let selected = roots.first().cloned();
        let sessions_root_display = selected
            .as_ref()
            .map(|root| format!("{}:{}", root.distro, root.sessions_root))
            .unwrap_or_else(|| "<none>".to_string());
        let sessions_root_available = selected.is_some();

        Self {
            runner,
            sessions_root_display,
            sessions_root_available,
            distro: selected.as_ref().map(|root| root.distro.clone()),
            sessions_root: selected.as_ref().map(|root| root.sessions_root.clone()),
            previous_sizes: HashMap::new(),
            active_key: None,
            offset: 0,
            pending: String::new(),
        }
    }

    fn unavailable(runner: R) -> Self {
        Self {
            runner,
            sessions_root_display: "<none>".to_string(),
            sessions_root_available: false,
            distro: None,
            sessions_root: None,
            previous_sizes: HashMap::new(),
            active_key: None,
            offset: 0,
            pending: String::new(),
        }
    }
}

impl<R: CommandRunner> SessionBackend for WslCommandBackend<R> {
    fn backend_kind(&self) -> &'static str {
        "wsl_command"
    }

    fn sessions_root_display(&self) -> &str {
        &self.sessions_root_display
    }

    fn sessions_root_available(&self) -> bool {
        self.sessions_root_available
    }

    fn select_active_session(&mut self) -> io::Result<Option<SessionCandidate>> {
        let Some(distro) = self.distro.clone() else {
            return Ok(None);
        };
        let Some(sessions_root) = self.sessions_root.clone() else {
            return Ok(None);
        };

        let find_command = format!(
            "if [ -d {root} ]; then find {root} -type f -name 'rollout-*.jsonl' -printf '%T@|%s|%p\\n'; fi",
            root = shell_quote(&sessions_root)
        );
        let output = run_wsl(
            &mut self.runner,
            &["-d", &distro, "--", "sh", "-lc", &find_command],
        )?;
        if !output.success {
            return Err(io::Error::other(format!(
                "wsl rollout scan failed: {}",
                output.stderr.trim()
            )));
        }

        let mut candidates =
            parse_rollout_candidates(&output.stdout, &distro, &mut self.previous_sizes);
        candidates.sort_by(|left, right| {
            right
                .had_recent_append
                .cmp(&left.had_recent_append)
                .then_with(|| right.modified_at.cmp(&left.modified_at))
                .then_with(|| right.path.cmp(&left.path))
        });
        Ok(candidates.into_iter().next())
    }

    fn read_new_lines(&mut self, session: &SessionCandidate) -> io::Result<Vec<String>> {
        let (distro, linux_path) = parse_session_key(&session.backend_key)?;
        if self.active_key.as_deref() != Some(&session.backend_key) {
            self.active_key = Some(session.backend_key.clone());
            self.offset = 0;
            self.pending.clear();
        }
        if session.size_bytes < self.offset {
            self.offset = 0;
            self.pending.clear();
        }

        let start = self.offset.saturating_add(1);
        let read_command = format!(
            "if [ -f {path} ]; then tail -c +{start} {path}; fi",
            path = shell_quote(linux_path),
        );
        let output = run_wsl(
            &mut self.runner,
            &["-d", distro, "--", "sh", "-lc", &read_command],
        )?;
        if !output.success {
            return Err(io::Error::other(format!(
                "wsl rollout read failed: {}",
                output.stderr.trim()
            )));
        }
        self.offset = session.size_bytes;

        if output.stdout.is_empty() {
            return Ok(Vec::new());
        }

        let mut combined = std::mem::take(&mut self.pending);
        combined.push_str(&output.stdout);
        let trailing_newline = combined.ends_with('\n');
        let mut lines: Vec<String> = combined.lines().map(ToString::to_string).collect();

        if !trailing_newline {
            self.pending = lines.pop().unwrap_or_default();
        }

        Ok(lines)
    }
}

fn run_wsl<R: CommandRunner>(runner: &mut R, args: &[&str]) -> io::Result<CommandOutput> {
    runner.run(args)
}

fn parse_lines(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'\''"#))
}

fn parse_rollout_candidates(
    stdout: &str,
    distro: &str,
    previous_sizes: &mut HashMap<String, u64>,
) -> Vec<SessionCandidate> {
    let mut candidates = Vec::new();
    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        let mut parts = line.splitn(3, '|');
        let modified = parts.next().and_then(|value| value.parse::<f64>().ok());
        let size = parts.next().and_then(|value| value.parse::<u64>().ok());
        let path = parts.next();
        let (Some(modified), Some(size_bytes), Some(path)) = (modified, size, path) else {
            continue;
        };
        let session_id = format!("{distro}:{path}");
        let previous_size = previous_sizes
            .get(&session_id)
            .copied()
            .unwrap_or(size_bytes);
        let had_recent_append = size_bytes > previous_size;
        previous_sizes.insert(session_id.clone(), size_bytes);

        let modified_at = UNIX_EPOCH + Duration::from_secs_f64(modified.max(0.0));
        candidates.push(SessionCandidate {
            session_id: session_id.clone(),
            backend_key: session_id.clone(),
            path: PathBuf::from(&session_id),
            modified_at,
            size_bytes,
            had_recent_append,
        });
    }
    candidates
}

fn parse_session_key(session_key: &str) -> io::Result<(&str, &str)> {
    session_key
        .split_once(':')
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid WSL session key"))
}

#[cfg(test)]
mod wsl_command_tests {
    use std::{collections::VecDeque, io};

    use super::*;
    use crate::codex::backend::command_runner::{CommandOutput, CommandRunner};

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
    fn prefers_a_sessions_root_matching_the_windows_username() {
        let mut runner = FakeRunner::default();
        runner.push_ok("Ubuntu\nDebian\n");
        runner.push_ok("/home/tester");
        runner.push_ok("ok");
        runner.push_ok("/home/bob");
        runner.push_ok("ok");

        let backend = WslCommandBackend::discover_with_runner(runner, Some("tester".to_string()));

        assert!(backend.sessions_root_available());
        assert_eq!(
            backend.sessions_root_display(),
            "Ubuntu:/home/tester/.codex/sessions"
        );
    }

    #[test]
    fn reads_rollout_candidates_and_incremental_lines() {
        let mut runner = FakeRunner::default();
        runner.push_ok("Ubuntu\n");
        runner.push_ok("/home/tester");
        runner.push_ok("ok");
        runner.push_ok("1710000000|10|/home/tester/.codex/sessions/2026/03/18/rollout-a.jsonl\n");
        runner.push_ok("{\"type\":\"event_msg\"}\n");

        let mut backend =
            WslCommandBackend::discover_with_runner(runner, Some("tester".to_string()));
        let session = backend
            .select_active_session()
            .expect("scan succeeds")
            .expect("session exists");

        assert_eq!(
            session.session_id,
            "Ubuntu:/home/tester/.codex/sessions/2026/03/18/rollout-a.jsonl"
        );
        assert_eq!(
            backend.read_new_lines(&session).expect("read succeeds"),
            vec!["{\"type\":\"event_msg\"}".to_string()]
        );
    }
}
