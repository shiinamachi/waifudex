use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime};

const SETTINGS_FILE_NAME: &str = "settings.json";
pub const APP_SETTINGS_CHANGED_EVENT: &str = "waifudex://app-settings-changed";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    pub always_on_top: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            always_on_top: true,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettingsUpdate {
    pub always_on_top: Option<bool>,
}

impl AppSettingsUpdate {
    fn apply_to(&self, settings: &mut AppSettings) {
        if let Some(always_on_top) = self.always_on_top {
            settings.always_on_top = always_on_top;
        }
    }
}

#[derive(Debug)]
struct AppSettingsStateInner {
    current: AppSettings,
    path: Option<PathBuf>,
}

#[derive(Debug)]
pub struct AppSettingsState {
    inner: Mutex<AppSettingsStateInner>,
}

impl Default for AppSettingsState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(AppSettingsStateInner {
                current: AppSettings::default(),
                path: None,
            }),
        }
    }
}

impl AppSettingsState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current(&self) -> AppSettings {
        self.inner
            .lock()
            .expect("app settings state mutex poisoned")
            .current
            .clone()
    }

    fn snapshot<R: Runtime>(&self, app: &AppHandle<R>) -> tauri::Result<(AppSettings, PathBuf)> {
        let inner = self
            .inner
            .lock()
            .expect("app settings state mutex poisoned");
        let path = match &inner.path {
            Some(path) => path.clone(),
            None => app_settings_path(app)?,
        };

        Ok((inner.current.clone(), path))
    }

    fn replace(&self, path: PathBuf, settings: AppSettings) {
        let mut inner = self
            .inner
            .lock()
            .expect("app settings state mutex poisoned");
        inner.path = Some(path);
        inner.current = settings;
    }
}

fn app_settings_changed_payload(settings: &AppSettings) -> AppSettings {
    settings.clone()
}

pub fn initialize<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let path = app_settings_path(app)?;
    let settings = load_app_settings_from_path(&path);
    let state = app.state::<AppSettingsState>();
    state.replace(path, settings.clone());
    crate::mascot_window::set_always_on_top(app, settings.always_on_top)?;
    Ok(())
}

pub fn current_app_settings<R: Runtime>(app: &AppHandle<R>) -> AppSettings {
    app.state::<AppSettingsState>().current()
}

pub fn update_app_settings<R: Runtime>(
    app: &AppHandle<R>,
    update: AppSettingsUpdate,
) -> tauri::Result<AppSettings> {
    let state = app.state::<AppSettingsState>();
    let (previous, path) = state.snapshot(app)?;
    let mut next = previous.clone();
    update.apply_to(&mut next);

    if next == previous {
        state.replace(path, previous.clone());
        return Ok(previous);
    }

    crate::mascot_window::set_always_on_top(app, next.always_on_top)?;

    if let Err(error) = persist_app_settings_to_path(&path, &next) {
        let _ = crate::mascot_window::set_always_on_top(app, previous.always_on_top);
        return Err(error);
    }

    state.replace(path, next.clone());
    let _ = crate::tray::sync_always_on_top_menu_item(app);
    let _ = app.emit(
        APP_SETTINGS_CHANGED_EVENT,
        app_settings_changed_payload(&next),
    );
    Ok(next)
}

#[tauri::command]
pub fn get_app_settings(app: AppHandle) -> AppSettings {
    current_app_settings(&app)
}

#[tauri::command]
pub fn update_app_settings_command(
    app: AppHandle,
    update: AppSettingsUpdate,
) -> Result<AppSettings, String> {
    update_app_settings(&app, update).map_err(|error| error.to_string())
}

fn app_settings_path<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<PathBuf> {
    Ok(app.path().app_config_dir()?.join(SETTINGS_FILE_NAME))
}

fn load_app_settings_from_path(path: &Path) -> AppSettings {
    match fs::read_to_string(path) {
        Ok(contents) => parse_app_settings_json(&contents),
        Err(error) if error.kind() == io::ErrorKind::NotFound => AppSettings::default(),
        Err(error) => {
            eprintln!(
                "failed to read app settings from {}: {error}",
                path.display()
            );
            AppSettings::default()
        }
    }
}

fn persist_app_settings_to_path(path: &Path, settings: &AppSettings) -> tauri::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let contents = serde_json::to_string_pretty(settings).map_err(io::Error::other)?;
    fs::write(path, format!("{contents}\n"))?;
    Ok(())
}

fn parse_app_settings_json(contents: &str) -> AppSettings {
    match serde_json::from_str::<AppSettings>(contents) {
        Ok(settings) => settings,
        Err(error) => {
            eprintln!("failed to parse app settings JSON, using defaults: {error}");
            AppSettings::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_settings_default_enables_always_on_top() {
        assert!(AppSettings::default().always_on_top);
    }

    #[test]
    fn invalid_settings_json_falls_back_to_defaults() {
        let settings = parse_app_settings_json("{invalid json");
        assert_eq!(settings, AppSettings::default());
    }

    #[test]
    fn app_settings_changed_event_name_is_stable() {
        assert_eq!(
            APP_SETTINGS_CHANGED_EVENT,
            "waifudex://app-settings-changed"
        );
    }

    #[test]
    fn app_settings_change_payload_serializes_camel_case_always_on_top() {
        let payload = app_settings_changed_payload(&AppSettings {
            always_on_top: false,
        });

        assert_eq!(
            serde_json::to_value(payload).unwrap(),
            serde_json::json!({
                "alwaysOnTop": false,
            })
        );
    }
}
