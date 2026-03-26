use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime};

const SETTINGS_FILE_NAME: &str = "settings.json";
pub const APP_SETTINGS_CHANGED_EVENT: &str = "waifudex://app-settings-changed";

const MIN_CHARACTER_SCALE: f64 = 0.5;
const MAX_CHARACTER_SCALE: f64 = 1.5;
pub const BASE_MASCOT_WIDTH: u32 = 420;
pub const BASE_MASCOT_HEIGHT: u32 = 720;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterWindowPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    pub always_on_top: bool,
    pub character_scale: f64,
    pub display_monitor_id: Option<String>,
    pub character_window_position: Option<CharacterWindowPosition>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            always_on_top: true,
            character_scale: 1.0,
            display_monitor_id: None,
            character_window_position: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettingsUpdate {
    pub always_on_top: Option<bool>,
    pub character_scale: Option<f64>,
    pub display_monitor_id: Option<String>,
    pub character_window_position: Option<CharacterWindowPosition>,
}

impl AppSettingsUpdate {
    fn apply_to(&self, settings: &mut AppSettings) {
        if let Some(always_on_top) = self.always_on_top {
            settings.always_on_top = always_on_top;
        }
        if let Some(character_scale) = self.character_scale {
            settings.character_scale =
                character_scale.clamp(MIN_CHARACTER_SCALE, MAX_CHARACTER_SCALE);
        }
        if let Some(display_monitor_id) = &self.display_monitor_id {
            settings.display_monitor_id = Some(display_monitor_id.clone());
        }
        if let Some(character_window_position) = self.character_window_position {
            settings.character_window_position = Some(character_window_position);
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

fn emit_app_settings_changed<R: Runtime>(app: &AppHandle<R>, settings: &AppSettings) {
    let _ = crate::tray::sync_always_on_top_menu_item(app);
    let _ = app.emit(
        APP_SETTINGS_CHANGED_EVENT,
        app_settings_changed_payload(settings),
    );
}

pub fn initialize<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let path = app_settings_path(app)?;
    let settings = load_app_settings_from_path(&path);
    let state = app.state::<AppSettingsState>();
    state.replace(path, settings.clone());
    crate::mascot_window::set_always_on_top(app, settings.always_on_top)?;

    let width = (BASE_MASCOT_WIDTH as f64 * settings.character_scale) as u32;
    let height = (BASE_MASCOT_HEIGHT as f64 * settings.character_scale) as u32;
    let _ = crate::mascot_window::resize(app, width, height);

    Ok(())
}

pub fn current_app_settings<R: Runtime>(app: &AppHandle<R>) -> AppSettings {
    app.state::<AppSettingsState>().current()
}

pub fn sync_display_monitor_on_startup<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let current = current_app_settings(app);
    let placement = crate::mascot_window::restore_window_placement(
        app,
        current.display_monitor_id.as_deref(),
        current.character_window_position,
    )?;
    if let Some(next) =
        merge_character_window_placement(&current, placement.monitor_id, placement.position)
    {
        persist_current_settings(app, next)?;
    }

    Ok(())
}

pub fn sync_display_monitor_from_window<R: Runtime>(
    app: &AppHandle<R>,
    display_monitor_id: Option<String>,
) -> tauri::Result<AppSettings> {
    let current = current_app_settings(app);
    sync_character_window_placement_from_window(
        app,
        display_monitor_id,
        current.character_window_position,
    )
}

pub fn sync_character_window_placement_from_window<R: Runtime>(
    app: &AppHandle<R>,
    display_monitor_id: Option<String>,
    position: Option<CharacterWindowPosition>,
) -> tauri::Result<AppSettings> {
    let current = current_app_settings(app);
    if let Some(next) = merge_character_window_placement(&current, display_monitor_id, position) {
        persist_current_settings(app, next)
    } else {
        Ok(current)
    }
}

fn resolve_window_move_update(
    previous: &AppSettings,
    mut next: AppSettings,
    placement: crate::mascot_window::MascotWindowPlacement,
) -> AppSettings {
    let resolved =
        merge_character_window_placement(previous, placement.monitor_id, placement.position)
            .unwrap_or_else(|| previous.clone());
    next.display_monitor_id = resolved.display_monitor_id;
    next.character_window_position = resolved.character_window_position;
    next
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

    if next.display_monitor_id != previous.display_monitor_id {
        match crate::mascot_window::move_to_monitor(app, next.display_monitor_id.as_deref()) {
            Ok(placement) => {
                next.display_monitor_id = placement.monitor_id;
                next.character_window_position = placement.position;
            }
            Err(error) => {
                let _ = crate::mascot_window::set_always_on_top(app, previous.always_on_top);
                return Err(error);
            }
        }
    }

    if (next.character_scale - previous.character_scale).abs() > f64::EPSILON {
        let next_width = (BASE_MASCOT_WIDTH as f64 * next.character_scale) as u32;
        let next_height = (BASE_MASCOT_HEIGHT as f64 * next.character_scale) as u32;
        if let Err(error) = crate::mascot_window::resize(app, next_width, next_height) {
            let _ = crate::mascot_window::set_always_on_top(app, previous.always_on_top);
            return Err(error);
        }
        if let Err(error) = crate::mascot::resize(app, next_width, next_height) {
            let prev_width = (BASE_MASCOT_WIDTH as f64 * previous.character_scale) as u32;
            let prev_height = (BASE_MASCOT_HEIGHT as f64 * previous.character_scale) as u32;
            let _ = crate::mascot_window::resize(app, prev_width, prev_height);
            let _ = crate::mascot_window::set_always_on_top(app, previous.always_on_top);
            return Err(error);
        }
    }

    if next.character_window_position != previous.character_window_position {
        let requested_position = next.character_window_position.unwrap_or(
            previous
                .character_window_position
                .unwrap_or(CharacterWindowPosition { x: 160, y: 160 }),
        );
        match crate::mascot_window::move_to_position(
            app,
            requested_position,
            next.display_monitor_id.as_deref(),
        ) {
            Ok(placement) => {
                next = resolve_window_move_update(&previous, next, placement);
            }
            Err(error) => {
                if next.display_monitor_id != previous.display_monitor_id {
                    let _ = crate::mascot_window::move_to_monitor(
                        app,
                        previous.display_monitor_id.as_deref(),
                    );
                }
                if (next.character_scale - previous.character_scale).abs() > f64::EPSILON {
                    let prev_width = (BASE_MASCOT_WIDTH as f64 * previous.character_scale) as u32;
                    let prev_height = (BASE_MASCOT_HEIGHT as f64 * previous.character_scale) as u32;
                    let _ = crate::mascot::resize(app, prev_width, prev_height);
                    let _ = crate::mascot_window::resize(app, prev_width, prev_height);
                }
                let _ = crate::mascot_window::set_always_on_top(app, previous.always_on_top);
                return Err(error);
            }
        }
    }

    if let Err(error) = persist_app_settings_to_path(&path, &next) {
        if next.display_monitor_id != previous.display_monitor_id {
            let _ =
                crate::mascot_window::move_to_monitor(app, previous.display_monitor_id.as_deref());
        }
        if next.character_window_position != previous.character_window_position {
            if let Some(previous_position) = previous.character_window_position {
                let _ = crate::mascot_window::move_to_position(
                    app,
                    previous_position,
                    previous.display_monitor_id.as_deref(),
                );
            }
        }
        if (next.character_scale - previous.character_scale).abs() > f64::EPSILON {
            let prev_width = (BASE_MASCOT_WIDTH as f64 * previous.character_scale) as u32;
            let prev_height = (BASE_MASCOT_HEIGHT as f64 * previous.character_scale) as u32;
            let _ = crate::mascot::resize(app, prev_width, prev_height);
            let _ = crate::mascot_window::resize(app, prev_width, prev_height);
        }
        let _ = crate::mascot_window::set_always_on_top(app, previous.always_on_top);
        return Err(error);
    }

    state.replace(path, next.clone());
    emit_app_settings_changed(app, &next);
    Ok(next)
}

fn merge_character_window_placement(
    current: &AppSettings,
    display_monitor_id: Option<String>,
    position: Option<CharacterWindowPosition>,
) -> Option<AppSettings> {
    if current.display_monitor_id == display_monitor_id
        && current.character_window_position == position
    {
        return None;
    }

    let mut next = current.clone();
    next.display_monitor_id = display_monitor_id;
    next.character_window_position = position;
    Some(next)
}

fn persist_current_settings<R: Runtime>(
    app: &AppHandle<R>,
    next: AppSettings,
) -> tauri::Result<AppSettings> {
    let state = app.state::<AppSettingsState>();
    let (_current, path) = state.snapshot(app)?;
    persist_app_settings_to_path(&path, &next)?;
    state.replace(path, next.clone());
    emit_app_settings_changed(app, &next);
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
        Ok(mut settings) => {
            settings.character_scale = settings
                .character_scale
                .clamp(MIN_CHARACTER_SCALE, MAX_CHARACTER_SCALE);
            settings
        }
        Err(error) => {
            eprintln!("failed to parse app settings JSON, using defaults: {error}");
            AppSettings::default()
        }
    }
}
