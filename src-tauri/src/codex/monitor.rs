use std::{
    ffi::OsStr,
    io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use tauri::{AppHandle, Emitter, Manager};

use crate::{
    codex::backend::{local_fs::LocalFsBackend, wsl_command::WslCommandBackend, SessionBackend},
    contracts::{
        RuntimeEvent, RuntimeEventPayload, RuntimeSnapshot, RUNTIME_EVENT_STREAM,
        RUNTIME_SNAPSHOT_EVENT,
    },
    runtime_state::RuntimeState,
};

use super::{
    liveness::{LivenessProbe, LivenessSnapshot},
    parser::parse_session_line,
    reducer::StatusReducer,
    snapshot_for_status, StatusKind,
};

const MONITOR_POLL_INTERVAL_MS: u64 = 250;

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
    backend: Box<dyn SessionBackend>,
    reducer: StatusReducer,
    last_status: Option<StatusKind>,
    last_material: Option<SnapshotMaterial>,
    active_session_path: Option<PathBuf>,
}

impl MonitorSupervisor {
    pub fn new(sessions_root: PathBuf, source: impl Into<String>) -> Self {
        Self::from_backend(LocalFsBackend::new(sessions_root), source)
    }

    pub fn from_backend(backend: impl SessionBackend + 'static, source: impl Into<String>) -> Self {
        Self::from_boxed_backend(Box::new(backend), source)
    }

    pub fn from_boxed_backend(backend: Box<dyn SessionBackend>, source: impl Into<String>) -> Self {
        let source = source.into();
        Self {
            source: source.clone(),
            backend,
            reducer: StatusReducer::new(source),
            last_status: None,
            last_material: None,
            active_session_path: None,
        }
    }

    pub(crate) fn tick(&mut self, liveness: LivenessSnapshot) -> io::Result<TickOutput> {
        self.tick_at(liveness, Instant::now())
    }

    pub(crate) fn tick_at(
        &mut self,
        liveness: LivenessSnapshot,
        now: Instant,
    ) -> io::Result<TickOutput> {
        let mut timeline_records = Vec::new();

        let snapshot_candidate = if !self.backend.sessions_root_available() {
            self.active_session_path = None;
            snapshot_for_status(
                StatusKind::CodexNotInstalled,
                self.source.clone(),
                None,
                self.backend.sessions_root_display().to_string(),
            )
        } else if let Some(candidate) = self.backend.select_active_session()? {
            self.active_session_path = Some(candidate.path.clone());
            let session_id = Some(candidate.session_id.clone());
            let lines = self.backend.read_new_lines(&candidate)?;

            if lines.is_empty() {
                self.reducer.reduce_at(
                    None,
                    session_id,
                    self.backend.sessions_root_display(),
                    liveness,
                    now,
                )
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
                    latest_snapshot = Some(self.reducer.reduce_at(
                        Some(&event),
                        session_id.clone(),
                        self.backend.sessions_root_display(),
                        liveness,
                        now,
                    ));
                }
                latest_snapshot.expect("non-empty line batch produced a snapshot")
            }
        } else {
            self.active_session_path = None;
            self.reducer.reduce_at(
                None,
                None,
                self.backend.sessions_root_display(),
                liveness,
                now,
            )
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

    fn maybe_refresh_backend(&mut self) {
        if !cfg!(windows) {
            return;
        }

        let should_refresh = !self.backend.sessions_root_available()
            || (self.backend.backend_kind() == "local_fs" && self.active_session_path.is_none());
        if !should_refresh {
            return;
        }

        let next_backend = select_session_backend();
        let backend_changed = self.backend.backend_kind() != next_backend.backend_kind()
            || self.backend.sessions_root_display() != next_backend.sessions_root_display();
        if backend_changed {
            self.backend = next_backend;
            self.active_session_path = None;
        }
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
        let backend = select_session_backend();
        let mut supervisor = MonitorSupervisor::from_boxed_backend(backend, "monitor");
        let mut probe = LivenessProbe::new("codex");
        let mut ticker = tokio::time::interval(Duration::from_millis(MONITOR_POLL_INTERVAL_MS));
        let mut last_logged_session: Option<PathBuf> = None;
        let mut last_logged_status: Option<StatusKind> = None;

        loop {
            ticker.tick().await;
            supervisor.maybe_refresh_backend();

            let runtime_state = app.state::<RuntimeState>();
            let liveness = probe.snapshot(
                supervisor.backend.backend_kind(),
                supervisor.backend.sessions_root_display(),
            );
            match collect_tick_emissions(&mut supervisor, &runtime_state, liveness) {
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
                if let Some(mascot) = app.try_state::<crate::mascot::MascotManager>() {
                    let _ = mascot.set_status(status);
                }
            }
        }
    });
}

fn select_session_backend() -> Box<dyn SessionBackend> {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let is_windows = cfg!(windows);
    let userprofile = std::env::var_os("USERPROFILE");
    let primary = default_sessions_root_from_env(
        std::env::var_os("HOME").as_deref(),
        userprofile.as_deref(),
        &current_dir,
        is_windows,
    );
    let local_observable = sessions_root_is_observable(&primary);

    let mut local_backend = LocalFsBackend::new(primary.clone());
    let local_has_active_session = local_backend
        .select_active_session()
        .ok()
        .flatten()
        .is_some();
    if local_observable && local_has_active_session {
        return Box::new(local_backend);
    }

    if is_windows {
        let preferred_user = userprofile
            .as_deref()
            .map(Path::new)
            .and_then(Path::file_name)
            .and_then(OsStr::to_str)
            .map(ToString::to_string);
        let wsl_backend = WslCommandBackend::discover(preferred_user);

        if wsl_backend.sessions_root_available() {
            return Box::new(wsl_backend);
        }

        local_backend = LocalFsBackend::new(primary.clone());
        return Box::new(local_backend);
    }

    Box::new(local_backend)
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

fn sessions_root_is_observable(root: &Path) -> bool {
    root.is_dir() && std::fs::read_dir(root).is_ok()
}

fn default_sessions_root_from_env(
    home: Option<&OsStr>,
    userprofile: Option<&OsStr>,
    current_dir: &Path,
    prefer_userprofile: bool,
) -> PathBuf {
    let primary = if prefer_userprofile {
        userprofile
            .map(PathBuf::from)
            .or_else(|| home.map(PathBuf::from))
    } else {
        home.map(PathBuf::from)
            .or_else(|| userprofile.map(PathBuf::from))
    };

    primary
        .unwrap_or_else(|| current_dir.to_path_buf())
        .join(".codex")
        .join("sessions")
}

fn status_label(status: StatusKind) -> &'static str {
    match status {
        StatusKind::Idle => "idle",
        StatusKind::CodexNotInstalled => "codex_not_installed",
        StatusKind::Thinking => "thinking",
        StatusKind::Coding => "coding",
        StatusKind::Question => "question",
        StatusKind::Complete => "complete",
    }
}
