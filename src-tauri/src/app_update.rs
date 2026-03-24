use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tauri_plugin_updater::UpdaterExt;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppUpdateStatus {
    Idle,
    Checking,
    Downloading,
    Installing,
    ReadyToRestart,
    UpToDate,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckTrigger {
    Startup,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateSnapshot {
    pub current_version: String,
    pub available_version: Option<String>,
    pub status: AppUpdateStatus,
    pub last_checked_at: Option<String>,
    pub last_error: Option<String>,
    pub should_prompt_restart: bool,
}

#[derive(Debug)]
pub struct AppUpdateCoordinator {
    snapshot: AppUpdateSnapshot,
    check_in_progress: bool,
}

impl AppUpdateCoordinator {
    pub fn new(current_version: impl Into<String>) -> Self {
        Self {
            snapshot: AppUpdateSnapshot {
                current_version: current_version.into(),
                available_version: None,
                status: AppUpdateStatus::Idle,
                last_checked_at: None,
                last_error: None,
                should_prompt_restart: false,
            },
            check_in_progress: false,
        }
    }

    pub fn snapshot(&self) -> AppUpdateSnapshot {
        self.snapshot.clone()
    }

    pub fn begin_check(&mut self, trigger: CheckTrigger) -> bool {
        if self.check_in_progress || self.snapshot.status == AppUpdateStatus::ReadyToRestart {
            return false;
        }

        self.check_in_progress = true;
        let _ = trigger;
        self.snapshot.status = AppUpdateStatus::Checking;
        self.snapshot.last_error = None;
        self.snapshot.last_checked_at = Some(now_timestamp());
        true
    }

    pub fn mark_downloading(&mut self, available_version: Option<String>) {
        self.snapshot.status = AppUpdateStatus::Downloading;
        self.snapshot.available_version = available_version;
        self.snapshot.last_error = None;
    }

    pub fn mark_installing(&mut self, available_version: Option<String>) {
        self.snapshot.status = AppUpdateStatus::Installing;
        self.snapshot.available_version = available_version;
        self.snapshot.last_error = None;
    }

    pub fn mark_ready_to_restart(&mut self, available_version: Option<String>) {
        self.check_in_progress = false;
        self.snapshot.status = AppUpdateStatus::ReadyToRestart;
        self.snapshot.available_version = available_version;
        self.snapshot.last_error = None;
        self.snapshot.last_checked_at = Some(now_timestamp());
        self.snapshot.should_prompt_restart = true;
    }

    pub fn mark_restart_prompt_deferred(&mut self) {
        if self.snapshot.status == AppUpdateStatus::ReadyToRestart {
            self.snapshot.should_prompt_restart = false;
        }
    }

    pub fn take_restart_prompt_request(&mut self) -> bool {
        if self.snapshot.status != AppUpdateStatus::ReadyToRestart
            || !self.snapshot.should_prompt_restart
        {
            return false;
        }

        self.snapshot.should_prompt_restart = false;
        true
    }

    pub fn complete_with_no_update(&mut self) {
        self.check_in_progress = false;
        self.snapshot.status = AppUpdateStatus::UpToDate;
        self.snapshot.available_version = None;
        self.snapshot.last_error = None;
        self.snapshot.last_checked_at = Some(now_timestamp());
        self.snapshot.should_prompt_restart = false;
    }

    pub fn mark_error(&mut self, message: impl Into<String>) {
        self.check_in_progress = false;
        self.snapshot.status = AppUpdateStatus::Error;
        self.snapshot.last_error = Some(message.into());
        self.snapshot.last_checked_at = Some(now_timestamp());
    }
}

#[derive(Debug)]
pub struct AppUpdateState {
    inner: Mutex<AppUpdateCoordinator>,
}

impl AppUpdateState {
    pub fn new(current_version: impl Into<String>) -> Self {
        Self {
            inner: Mutex::new(AppUpdateCoordinator::new(current_version)),
        }
    }

    pub fn snapshot(&self) -> AppUpdateSnapshot {
        self.inner
            .lock()
            .expect("app update state mutex poisoned")
            .snapshot()
    }

    fn update<F, T>(&self, mut f: F) -> T
    where
        F: FnMut(&mut AppUpdateCoordinator) -> T,
    {
        let mut inner = self.inner.lock().expect("app update state mutex poisoned");
        f(&mut inner)
    }

    fn begin_check(&self, trigger: CheckTrigger) -> bool {
        self.update(|inner| inner.begin_check(trigger))
    }

    fn mark_downloading(&self, available_version: Option<String>) {
        self.update(|inner| inner.mark_downloading(available_version.clone()));
    }

    fn mark_installing(&self, available_version: Option<String>) {
        self.update(|inner| inner.mark_installing(available_version.clone()));
    }

    fn mark_ready_to_restart(&self, available_version: Option<String>) {
        self.update(|inner| inner.mark_ready_to_restart(available_version.clone()));
    }

    pub fn mark_restart_prompt_deferred(&self) {
        self.update(AppUpdateCoordinator::mark_restart_prompt_deferred);
    }

    fn take_restart_prompt_request(&self) -> bool {
        self.update(AppUpdateCoordinator::take_restart_prompt_request)
    }

    fn complete_with_no_update(&self) {
        self.update(AppUpdateCoordinator::complete_with_no_update);
    }

    fn mark_error(&self, message: impl Into<String>) {
        let message = message.into();
        self.update(|inner| inner.mark_error(message.clone()));
    }
}

fn now_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("Rfc3339 timestamp should format")
}

pub fn start_startup_check<R: Runtime>(app: AppHandle<R>) {
    start_check(app, CheckTrigger::Startup);
}

pub fn is_update_ready<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.state::<AppUpdateState>().snapshot().status == AppUpdateStatus::ReadyToRestart
}

fn start_check<R: Runtime>(app: AppHandle<R>, trigger: CheckTrigger) {
    let state = app.state::<AppUpdateState>();
    if !state.begin_check(trigger) {
        return;
    }

    tauri::async_runtime::spawn(async move {
        run_update_check(app).await;
    });
}

async fn run_update_check<R: Runtime>(app: AppHandle<R>) {
    let updater = match app.updater() {
        Ok(updater) => updater,
        Err(error) => {
            app.state::<AppUpdateState>().mark_error(error.to_string());
            return;
        }
    };

    let update = match updater.check().await {
        Ok(update) => update,
        Err(error) => {
            app.state::<AppUpdateState>().mark_error(error.to_string());
            return;
        }
    };

    let Some(update) = update else {
        app.state::<AppUpdateState>().complete_with_no_update();
        return;
    };

    let version = update.version.clone();
    app.state::<AppUpdateState>()
        .mark_downloading(Some(version.clone()));

    let install_state = app.clone();
    let install_version = version.clone();
    match update
        .download_and_install(
            |_chunk_length, _content_length| {},
            move || {
                install_state
                    .state::<AppUpdateState>()
                    .mark_installing(Some(install_version.clone()));
            },
        )
        .await
    {
        Ok(()) => {
            app.state::<AppUpdateState>()
                .mark_ready_to_restart(Some(version));
            let _ = crate::tray::sync_update_restart_menu_item(&app);
            prompt_for_restart_if_needed(app);
        }
        Err(error) => {
            app.state::<AppUpdateState>().mark_error(error.to_string());
            let _ = crate::tray::sync_update_restart_menu_item(&app);
        }
    }
}

#[tauri::command]
pub fn get_app_update_state(state: State<'_, AppUpdateState>) -> AppUpdateSnapshot {
    state.snapshot()
}

#[tauri::command]
pub fn check_for_updates_command<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppUpdateState>,
) -> AppUpdateSnapshot {
    start_check(app, CheckTrigger::Manual);
    state.snapshot()
}

#[tauri::command]
pub fn restart_to_apply_update_command<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    restart_to_apply_update(app)
}

pub fn restart_to_apply_update<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    if !is_update_ready(&app) {
        return Err("no installed update is waiting to restart".to_string());
    }

    app.restart();
}

fn prompt_for_restart_if_needed<R: Runtime>(app: AppHandle<R>) {
    if !app.state::<AppUpdateState>().take_restart_prompt_request() {
        return;
    }

    std::thread::spawn(move || {
        let should_restart = app
            .dialog()
            .message("An update is ready. Restart now to apply it.")
            .title("Update Ready")
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Restart now".to_string(),
                "Later".to_string(),
            ))
            .blocking_show();

        if should_restart {
            let _ = restart_to_apply_update(app);
        } else {
            app.state::<AppUpdateState>().mark_restart_prompt_deferred();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_checks_are_ignored_while_a_check_is_in_progress() {
        let mut updater = AppUpdateCoordinator::new("0.1.0");

        assert!(updater.begin_check(CheckTrigger::Startup));
        assert!(!updater.begin_check(CheckTrigger::Manual));
        assert_eq!(updater.snapshot().status, AppUpdateStatus::Checking);
    }

    #[test]
    fn ready_to_restart_survives_later_choice() {
        let mut updater = AppUpdateCoordinator::new("0.1.0");

        updater.mark_ready_to_restart(Some("0.2.0".to_string()));
        updater.mark_restart_prompt_deferred();

        let snapshot = updater.snapshot();
        assert_eq!(snapshot.status, AppUpdateStatus::ReadyToRestart);
        assert_eq!(snapshot.available_version.as_deref(), Some("0.2.0"));
        assert!(!snapshot.should_prompt_restart);
    }

    #[test]
    fn ready_to_restart_blocks_future_checks_and_stays_sticky() {
        let mut updater = AppUpdateCoordinator::new("0.1.0");

        updater.mark_ready_to_restart(Some("0.2.0".to_string()));
        updater.mark_restart_prompt_deferred();

        assert!(!updater.begin_check(CheckTrigger::Manual));

        let snapshot = updater.snapshot();
        assert_eq!(snapshot.status, AppUpdateStatus::ReadyToRestart);
        assert_eq!(snapshot.available_version.as_deref(), Some("0.2.0"));
        assert!(!snapshot.should_prompt_restart);
    }

    #[test]
    fn restart_prompt_is_triggered_once_when_update_becomes_ready() {
        let mut updater = AppUpdateCoordinator::new("0.1.0");

        assert!(!updater.take_restart_prompt_request());

        updater.mark_ready_to_restart(Some("0.2.0".to_string()));

        assert!(updater.take_restart_prompt_request());
        assert!(!updater.take_restart_prompt_request());
    }

    #[test]
    fn snapshot_exposes_expected_status_transitions() {
        let mut updater = AppUpdateCoordinator::new("0.1.0");

        assert_eq!(updater.snapshot().status, AppUpdateStatus::Idle);

        assert!(updater.begin_check(CheckTrigger::Startup));
        assert_eq!(updater.snapshot().status, AppUpdateStatus::Checking);

        updater.mark_downloading(Some("0.2.0".to_string()));
        assert_eq!(updater.snapshot().status, AppUpdateStatus::Downloading);

        updater.mark_installing(Some("0.2.0".to_string()));
        assert_eq!(updater.snapshot().status, AppUpdateStatus::Installing);

        updater.mark_ready_to_restart(Some("0.2.0".to_string()));
        assert_eq!(updater.snapshot().status, AppUpdateStatus::ReadyToRestart);

        updater.complete_with_no_update();
        assert_eq!(updater.snapshot().status, AppUpdateStatus::UpToDate);

        updater.mark_error("network failure");
        let snapshot = updater.snapshot();
        assert_eq!(snapshot.status, AppUpdateStatus::Error);
        assert_eq!(snapshot.last_error.as_deref(), Some("network failure"));
    }
}
