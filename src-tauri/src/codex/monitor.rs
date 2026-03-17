use std::{io, path::PathBuf, time::Duration};

use tauri::{AppHandle, Emitter, Manager};

use super::{
    discovery::SessionDiscovery,
    liveness::{LivenessProbe, LivenessSnapshot},
    parser::parse_session_line,
    reducer::StatusReducer,
    session_reader::SessionReader,
    StatusKind, StatusPayload, CODEX_STATUS_EVENT,
};

pub struct MonitorSupervisor {
    discovery: SessionDiscovery,
    reader: SessionReader,
    reducer: StatusReducer,
    last_status: Option<StatusKind>,
    active_session_path: Option<PathBuf>,
}

impl MonitorSupervisor {
    pub fn new(sessions_root: PathBuf, source: impl Into<String>) -> Self {
        Self {
            discovery: SessionDiscovery::new(sessions_root),
            reader: SessionReader::new(),
            reducer: StatusReducer::new(source),
            last_status: None,
            active_session_path: None,
        }
    }

    pub fn tick(&mut self, liveness: LivenessSnapshot) -> io::Result<Option<StatusPayload>> {
        let payload = if let Some(candidate) = self.discovery.select_active_session()? {
            self.active_session_path = Some(candidate.path.clone());
            let lines = self.reader.read_new_lines(&candidate.path)?;
            if lines.is_empty() {
                self.reducer.reduce(None, liveness)
            } else {
                let mut latest_payload = None;
                for line in lines {
                    let event = parse_session_line(&line);
                    latest_payload = Some(self.reducer.reduce(Some(&event), liveness));
                }
                latest_payload.expect("non-empty line batch produced a payload")
            }
        } else {
            self.active_session_path = None;
            self.reducer.reduce(None, liveness)
        };

        if self.last_status == Some(payload.status) {
            return Ok(None);
        }

        self.last_status = Some(payload.status);
        Ok(Some(payload))
    }

    pub fn current_status(&self) -> Option<StatusKind> {
        self.last_status
    }

    pub fn current_session_path(&self) -> Option<&std::path::Path> {
        self.active_session_path.as_deref()
    }
}

pub fn start_monitor(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut supervisor = MonitorSupervisor::new(default_sessions_root(), "monitor");
        let probe = LivenessProbe::new("codex");
        let mut window_policy = crate::window::WindowVisibilityPolicy::new(2);
        let mut ticker = tokio::time::interval(Duration::from_secs(1));
        let mut last_logged_session: Option<PathBuf> = None;
        let mut last_logged_status: Option<StatusKind> = None;

        loop {
            ticker.tick().await;

            match supervisor.tick(probe.snapshot()) {
                Ok(Some(payload)) => {
                    if let Some(message) = format_status_transition_log(
                        last_logged_status,
                        payload.status,
                        supervisor.current_session_path(),
                    ) {
                        eprintln!("{message}");
                    }
                    last_logged_status = Some(payload.status);
                    let _ = app.emit(CODEX_STATUS_EVENT, payload);
                }
                Ok(None) => {}
                Err(error) => {
                    eprintln!("waifudex monitor tick failed: {error}");
                }
            }

            let current_session = supervisor.current_session_path().map(PathBuf::from);
            if let Some(message) = format_session_transition_log(
                last_logged_session.as_deref(),
                current_session.as_deref(),
                supervisor.current_status(),
            ) {
                eprintln!("{message}");
            }
            last_logged_session = current_session;

            if let Some(status) = supervisor.current_status() {
                if let Some(window) = app.get_webview_window("main") {
                    if let Ok(visible) = window.is_visible() {
                        window_policy.sync_visible(visible);
                    }
                }

                match window_policy.on_status(status) {
                    crate::window::WindowCommand::Show => {
                        let _ = crate::window::show_main_window(&app);
                    }
                    crate::window::WindowCommand::Hide => {
                        let _ = crate::window::hide_main_window(&app);
                    }
                    crate::window::WindowCommand::Noop => {}
                }
            }
        }
    });
}

pub(crate) fn format_session_transition_log(
    previous: Option<&std::path::Path>,
    current: Option<&std::path::Path>,
    status: Option<StatusKind>,
) -> Option<String> {
    if previous == current {
        return None;
    }

    match (previous, current) {
        (_, Some(path)) => Some(format!(
            "waifudex monitor: active session detected path={} status={}",
            path.display(),
            status.map(status_label).unwrap_or("unknown")
        )),
        (Some(path), None) => Some(format!(
            "waifudex monitor: session cleared path={} status={}",
            path.display(),
            status.map(status_label).unwrap_or("unknown")
        )),
        (None, None) => None,
    }
}

pub(crate) fn format_status_transition_log(
    previous: Option<StatusKind>,
    current: StatusKind,
    session_path: Option<&std::path::Path>,
) -> Option<String> {
    if previous == Some(current) {
        return None;
    }

    let session_path = session_path
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<none>".to_string());

    Some(format!(
        "waifudex monitor: status changed {} -> {} path={}",
        previous.map(status_label).unwrap_or("unknown"),
        status_label(current),
        session_path
    ))
}

fn default_sessions_root() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
        .join("sessions")
}

fn status_label(status: StatusKind) -> &'static str {
    match status {
        StatusKind::Idle => "idle",
        StatusKind::Thinking => "thinking",
        StatusKind::Writing => "writing",
        StatusKind::RunningTests => "running_tests",
        StatusKind::Success => "success",
        StatusKind::Error => "error",
    }
}

#[cfg(test)]
mod monitor_tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::codex::{
        liveness::LivenessSnapshot,
        monitor::{format_session_transition_log, format_status_transition_log, MonitorSupervisor},
        StatusKind,
    };

    #[test]
    fn detects_a_new_rollout_file_after_startup() {
        let root = create_temp_root();
        let mut supervisor = MonitorSupervisor::new(root.clone(), "monitor");

        let initial = supervisor
            .tick(LivenessSnapshot::offline())
            .expect("initial tick succeeds")
            .expect("initial payload exists");
        assert_eq!(initial.status, StatusKind::Idle);

        let rollout = root.join("2026/03/17/rollout-late.jsonl");
        write_file(
            &rollout,
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\",\"turn_id\":\"abc\"}}\n",
        );

        let payload = supervisor
            .tick(LivenessSnapshot::offline())
            .expect("follow-up tick succeeds")
            .expect("new rollout payload exists");

        assert_eq!(payload.status, StatusKind::Thinking);

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn switches_from_idle_to_active_when_new_lines_are_appended() {
        let root = create_temp_root();
        let rollout = root.join("2026/03/17/rollout-active.jsonl");
        write_file(&rollout, "");

        let mut supervisor = MonitorSupervisor::new(root.clone(), "monitor");
        let initial = supervisor
            .tick(LivenessSnapshot::offline())
            .expect("initial tick succeeds")
            .expect("initial payload exists");
        assert_eq!(initial.status, StatusKind::Idle);

        append_file(
            &rollout,
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\",\"turn_id\":\"abc\"}}\n",
        );
        append_file(
            &rollout,
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"name\":\"cargo test\",\"arguments\":\"{}\",\"call_id\":\"call_123\"}}\n",
        );

        let payload = supervisor
            .tick(LivenessSnapshot::online())
            .expect("append tick succeeds")
            .expect("active payload exists");

        assert_eq!(payload.status, StatusKind::RunningTests);

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn returns_to_idle_after_inactivity_with_no_live_process() {
        let root = create_temp_root();
        let rollout = root.join("2026/03/17/rollout-idle.jsonl");
        write_file(
            &rollout,
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\",\"turn_id\":\"abc\"}}\n",
        );

        let mut supervisor = MonitorSupervisor::new(root.clone(), "monitor");
        let active = supervisor
            .tick(LivenessSnapshot::online())
            .expect("active tick succeeds")
            .expect("active payload exists");
        assert_eq!(active.status, StatusKind::Thinking);

        let idle = supervisor
            .tick(LivenessSnapshot::offline())
            .expect("idle tick succeeds")
            .expect("idle payload exists");

        assert_eq!(idle.status, StatusKind::Idle);
        assert!(supervisor.current_session_path().is_some());

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn keeps_tool_state_when_multiple_events_arrive_in_one_poll() {
        let root = create_temp_root();
        let rollout = root.join("2026/03/17/rollout-batch.jsonl");
        write_file(
            &rollout,
            concat!(
                "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\",\"turn_id\":\"abc\"}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"name\":\"cargo test\",\"arguments\":\"{}\",\"call_id\":\"call_123\"}}\n",
                "{\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\"}}\n"
            ),
        );

        let mut supervisor = MonitorSupervisor::new(root.clone(), "monitor");
        let payload = supervisor
            .tick(LivenessSnapshot::online())
            .expect("batch tick succeeds")
            .expect("batch payload exists");

        assert_eq!(payload.status, StatusKind::RunningTests);

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn tracks_the_active_session_path_and_clears_it_when_sessions_disappear() {
        let root = create_temp_root();
        let rollout = root.join("2026/03/17/rollout-track.jsonl");
        write_file(
            &rollout,
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\",\"turn_id\":\"abc\"}}\n",
        );

        let mut supervisor = MonitorSupervisor::new(root.clone(), "monitor");
        let _ = supervisor
            .tick(LivenessSnapshot::online())
            .expect("initial tick succeeds");

        assert_eq!(supervisor.current_session_path(), Some(rollout.as_path()));

        fs::remove_file(&rollout).expect("remove rollout");

        let _ = supervisor
            .tick(LivenessSnapshot::offline())
            .expect("follow-up tick succeeds");

        assert!(supervisor.current_session_path().is_none());

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn formats_a_log_message_when_a_session_is_detected() {
        let message = format_session_transition_log(
            None,
            Some(Path::new("/tmp/rollout-123.jsonl")),
            Some(StatusKind::Thinking),
        )
        .expect("session detection log");

        assert!(message.contains("active session detected"));
        assert!(message.contains("/tmp/rollout-123.jsonl"));
        assert!(message.contains("thinking"));
    }

    #[test]
    fn formats_a_log_message_when_a_session_disappears() {
        let message = format_session_transition_log(
            Some(Path::new("/tmp/rollout-123.jsonl")),
            None,
            Some(StatusKind::Idle),
        )
        .expect("session cleared log");

        assert!(message.contains("session cleared"));
        assert!(message.contains("/tmp/rollout-123.jsonl"));
    }

    #[test]
    fn formats_a_log_message_when_status_changes_inside_the_same_session() {
        let message = format_status_transition_log(
            Some(StatusKind::Idle),
            StatusKind::Thinking,
            Some(Path::new("/tmp/rollout-123.jsonl")),
        )
        .expect("status transition log");

        assert!(message.contains("status changed"));
        assert!(message.contains("idle -> thinking"));
        assert!(message.contains("/tmp/rollout-123.jsonl"));
    }

    fn create_temp_root() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("waifudex-monitor-{unique}"));
        fs::create_dir_all(&root).expect("create temp root");
        root
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent directories");
        }
        fs::write(path, contents).expect("write temp file");
    }

    fn append_file(path: &Path, contents: &str) {
        use std::io::Write;

        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(path)
            .expect("open temp file for append");
        file.write_all(contents.as_bytes())
            .expect("append temp file");
    }
}
