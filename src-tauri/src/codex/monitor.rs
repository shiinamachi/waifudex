use std::{io, path::PathBuf, time::Duration};

use tauri::{AppHandle, Emitter, Manager};

use crate::{
    contracts::{
        RuntimeEvent, RuntimeEventPayload, RuntimeSnapshot, RUNTIME_EVENT_STREAM,
        RUNTIME_SNAPSHOT_EVENT,
    },
    runtime_state::RuntimeState,
};

use super::{
    discovery::SessionDiscovery,
    liveness::{LivenessProbe, LivenessSnapshot},
    parser::parse_session_line,
    reducer::StatusReducer,
    session_reader::SessionReader,
    StatusKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct TimelineRecord {
    session_id: Option<String>,
    payload: RuntimeEventPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SnapshotMaterial {
    status: StatusKind,
    session_id: Option<String>,
    summary: String,
    detail: String,
    source: String,
}

impl SnapshotMaterial {
    fn from_snapshot(snapshot: &RuntimeSnapshot) -> Self {
        Self {
            status: snapshot.status,
            session_id: snapshot.session_id.clone(),
            summary: snapshot.summary.clone(),
            detail: snapshot.detail.clone(),
            source: snapshot.source.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TickOutput {
    snapshot: Option<RuntimeSnapshot>,
    timeline_records: Vec<TimelineRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TickEmissions {
    pub snapshot: Option<RuntimeSnapshot>,
    pub timeline: Vec<RuntimeEvent>,
}

pub struct MonitorSupervisor {
    source: String,
    discovery: SessionDiscovery,
    reader: SessionReader,
    reducer: StatusReducer,
    last_status: Option<StatusKind>,
    last_material: Option<SnapshotMaterial>,
    active_session_path: Option<PathBuf>,
}

impl MonitorSupervisor {
    pub fn new(sessions_root: PathBuf, source: impl Into<String>) -> Self {
        let source = source.into();
        Self {
            source: source.clone(),
            discovery: SessionDiscovery::new(sessions_root),
            reader: SessionReader::new(),
            reducer: StatusReducer::new(source),
            last_status: None,
            last_material: None,
            active_session_path: None,
        }
    }

    pub(crate) fn tick(&mut self, liveness: LivenessSnapshot) -> io::Result<TickOutput> {
        let mut timeline_records = Vec::new();

        let snapshot_candidate = if let Some(candidate) = self.discovery.select_active_session()? {
            self.active_session_path = Some(candidate.path.clone());
            let session_id = Some(candidate.path.display().to_string());
            let lines = self.reader.read_new_lines(&candidate.path)?;

            if lines.is_empty() {
                self.reducer.reduce(None, session_id, liveness)
            } else {
                let mut latest_snapshot = None;
                for line in lines {
                    let event = parse_session_line(&line);
                    timeline_records.push(TimelineRecord {
                        session_id: session_id.clone(),
                        payload: RuntimeEventPayload {
                            raw_line: line,
                            parsed_type: event.parsed_type().map(ToString::to_string),
                            parse_ok: event.parse_ok(),
                        },
                    });
                    latest_snapshot = Some(self.reducer.reduce(
                        Some(&event),
                        session_id.clone(),
                        liveness,
                    ));
                }
                latest_snapshot.expect("non-empty line batch produced a snapshot")
            }
        } else {
            self.active_session_path = None;
            self.reducer.reduce(None, None, liveness)
        };

        let material = SnapshotMaterial::from_snapshot(&snapshot_candidate);
        let snapshot = if self.last_material == Some(material.clone()) {
            None
        } else {
            self.last_material = Some(material);
            self.last_status = Some(snapshot_candidate.status);
            Some(snapshot_candidate)
        };

        Ok(TickOutput {
            snapshot,
            timeline_records,
        })
    }

    pub fn current_status(&self) -> Option<StatusKind> {
        self.last_status
    }

    pub fn current_session_path(&self) -> Option<&std::path::Path> {
        self.active_session_path.as_deref()
    }
}

pub(crate) fn collect_tick_emissions(
    supervisor: &mut MonitorSupervisor,
    state: &RuntimeState,
    liveness: LivenessSnapshot,
) -> io::Result<TickEmissions> {
    let output = supervisor.tick(liveness)?;
    let snapshot = output
        .snapshot
        .map(|snapshot| state.record_snapshot(snapshot));
    let timeline = output
        .timeline_records
        .into_iter()
        .map(|record| {
            let cursor = state.next_timeline_event(record.session_id.as_deref());
            RuntimeEvent {
                event_id: cursor.event_id,
                session_id: record.session_id,
                sequence: cursor.sequence,
                received_at: super::timestamp_now(),
                source: supervisor.source.clone(),
                kind: "session_line".to_string(),
                payload: record.payload,
            }
        })
        .collect();

    Ok(TickEmissions { snapshot, timeline })
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

            let runtime_state = app.state::<RuntimeState>();
            match collect_tick_emissions(&mut supervisor, &runtime_state, probe.snapshot()) {
                Ok(emissions) => {
                    for timeline_event in emissions.timeline {
                        let _ = app.emit(RUNTIME_EVENT_STREAM, timeline_event);
                    }

                    if let Some(snapshot) = emissions.snapshot {
                        if let Some(message) = format_status_transition_log(
                            last_logged_status,
                            snapshot.status,
                            supervisor.current_session_path(),
                        ) {
                            eprintln!("{message}");
                        }
                        last_logged_status = Some(snapshot.status);
                        let _ = app.emit(RUNTIME_SNAPSHOT_EVENT, snapshot);
                    }
                }
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
        monitor::{
            collect_tick_emissions, format_session_transition_log, format_status_transition_log,
            MonitorSupervisor,
        },
        StatusKind,
    };
    use crate::runtime_state::RuntimeState;

    #[test]
    fn updates_bootstrap_state_when_first_snapshot_is_produced() {
        let root = create_temp_root();
        let mut supervisor = MonitorSupervisor::new(root.clone(), "monitor");
        let state = RuntimeState::new();

        let emissions =
            collect_tick_emissions(&mut supervisor, &state, LivenessSnapshot::offline())
                .expect("tick collection succeeds");

        assert_eq!(
            emissions
                .snapshot
                .as_ref()
                .expect("snapshot should exist")
                .status,
            StatusKind::Idle
        );
        assert!(
            emissions.timeline.is_empty(),
            "no timeline rows expected on empty startup"
        );

        let bootstrap = state
            .bootstrap()
            .snapshot
            .expect("bootstrap snapshot should be recorded");
        assert_eq!(bootstrap.status, StatusKind::Idle);
        assert_eq!(bootstrap.revision, 0);

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn emits_snapshot_only_when_state_changes_materially() {
        let root = create_temp_root();
        let rollout = root.join("2026/03/17/rollout-material.jsonl");
        write_file(&rollout, "");

        let mut supervisor = MonitorSupervisor::new(root.clone(), "monitor");
        let state = RuntimeState::new();

        let first = collect_tick_emissions(&mut supervisor, &state, LivenessSnapshot::offline())
            .expect("first tick succeeds");
        assert!(first.snapshot.is_some(), "first idle snapshot should emit");

        append_file(
            &rollout,
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\",\"turn_id\":\"abc\"}}\n",
        );
        let second = collect_tick_emissions(&mut supervisor, &state, LivenessSnapshot::online())
            .expect("second tick succeeds");
        assert_eq!(
            second
                .snapshot
                .as_ref()
                .expect("thinking snapshot should emit")
                .status,
            StatusKind::Thinking
        );

        let third = collect_tick_emissions(&mut supervisor, &state, LivenessSnapshot::online())
            .expect("third tick succeeds");
        assert!(
            third.snapshot.is_none(),
            "snapshot should not emit while status is unchanged"
        );

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn emits_snapshot_and_updates_bootstrap_when_session_changes_with_same_status() {
        let root = create_temp_root();
        let rollout_a = root.join("2026/03/17/rollout-session-a.jsonl");
        write_file(
            &rollout_a,
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\",\"turn_id\":\"a\"}}\n",
        );

        let mut supervisor = MonitorSupervisor::new(root.clone(), "monitor");
        let state = RuntimeState::new();

        let first = collect_tick_emissions(&mut supervisor, &state, LivenessSnapshot::online())
            .expect("first tick succeeds");
        let first_snapshot = first.snapshot.expect("first snapshot should emit");
        assert_eq!(first_snapshot.status, StatusKind::Thinking);
        assert_eq!(
            first_snapshot.session_id.as_deref(),
            Some(rollout_a.display().to_string().as_str())
        );

        let rollout_b = root.join("2026/03/17/rollout-session-b.jsonl");
        write_file(
            &rollout_b,
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\",\"turn_id\":\"b\"}}\n",
        );

        let second = collect_tick_emissions(&mut supervisor, &state, LivenessSnapshot::online())
            .expect("second tick succeeds");
        let second_snapshot = second
            .snapshot
            .expect("snapshot should emit when session changes");
        assert_eq!(second_snapshot.status, StatusKind::Thinking);
        assert_eq!(
            second_snapshot.session_id.as_deref(),
            Some(rollout_b.display().to_string().as_str())
        );

        let bootstrap = state
            .bootstrap()
            .snapshot
            .expect("bootstrap snapshot should exist");
        assert_eq!(bootstrap.status, StatusKind::Thinking);
        assert_eq!(
            bootstrap.session_id.as_deref(),
            Some(rollout_b.display().to_string().as_str())
        );

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn emits_timeline_events_for_each_new_line_with_increasing_sequence() {
        let root = create_temp_root();
        let rollout = root.join("2026/03/17/rollout-timeline.jsonl");
        write_file(&rollout, "");

        let mut supervisor = MonitorSupervisor::new(root.clone(), "monitor");
        let state = RuntimeState::new();
        let _ = collect_tick_emissions(&mut supervisor, &state, LivenessSnapshot::offline())
            .expect("prime tick succeeds");

        append_file(
            &rollout,
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\",\"turn_id\":\"abc\"}}\n",
        );
        append_file(
            &rollout,
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"name\":\"cargo test\",\"arguments\":\"{}\",\"call_id\":\"call_123\"}}\n",
        );

        let batch = collect_tick_emissions(&mut supervisor, &state, LivenessSnapshot::online())
            .expect("timeline tick succeeds");

        assert_eq!(batch.timeline.len(), 2);
        assert_eq!(batch.timeline[0].sequence, 0);
        assert_eq!(batch.timeline[1].sequence, 1);
        assert_eq!(
            batch.timeline[0].payload.raw_line,
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\",\"turn_id\":\"abc\"}}"
        );
        assert_eq!(
            batch.timeline[1].payload.raw_line,
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"name\":\"cargo test\",\"arguments\":\"{}\",\"call_id\":\"call_123\"}}"
        );
        assert_eq!(
            batch.timeline[0].payload.parsed_type.as_deref(),
            Some("task_started")
        );
        assert_eq!(
            batch.timeline[1].payload.parsed_type.as_deref(),
            Some("tool_call_started")
        );
        assert!(batch.timeline[0].payload.parse_ok);
        assert!(batch.timeline[1].payload.parse_ok);

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn detects_a_new_rollout_file_after_startup() {
        let root = create_temp_root();
        let mut supervisor = MonitorSupervisor::new(root.clone(), "monitor");

        let initial = supervisor
            .tick(LivenessSnapshot::offline())
            .expect("initial tick succeeds")
            .snapshot
            .expect("initial snapshot exists");
        assert_eq!(initial.status, StatusKind::Idle);

        let rollout = root.join("2026/03/17/rollout-late.jsonl");
        write_file(
            &rollout,
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\",\"turn_id\":\"abc\"}}\n",
        );

        let payload = supervisor
            .tick(LivenessSnapshot::offline())
            .expect("follow-up tick succeeds")
            .snapshot
            .expect("new rollout snapshot exists");

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
            .snapshot
            .expect("initial snapshot exists");
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
            .snapshot
            .expect("active snapshot exists");

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
            .snapshot
            .expect("active snapshot exists");
        assert_eq!(active.status, StatusKind::Thinking);

        let idle = supervisor
            .tick(LivenessSnapshot::offline())
            .expect("idle tick succeeds")
            .snapshot
            .expect("idle snapshot exists");

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
            .snapshot
            .expect("batch snapshot exists");

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
