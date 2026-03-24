use std::sync::Mutex;

use crate::codex::StatusKind;
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

pub const SETTINGS_WINDOW_LABEL: &str = "settings";
pub const CHARACTER_VISIBILITY_CHANGED_EVENT: &str = "waifudex://character-visibility-changed";
const SETTINGS_WINDOW_TITLE: &str = "Settings - waifudex";
const SETTINGS_WINDOW_ENTRY: &str = "index.html";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsWindowAction {
    ShowExisting,
    CreateNew,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowCommand {
    Show,
    Hide,
    Noop,
}

#[derive(Debug, Clone)]
pub struct WindowVisibilityPolicy {
    manual_open: bool,
    manual_hidden: bool,
    visible: bool,
}

pub struct WindowVisibilityState {
    inner: Mutex<WindowVisibilityPolicy>,
}

fn emit_character_visibility_changed<R: Runtime>(
    app: &AppHandle<R>,
    visible: bool,
) -> tauri::Result<()> {
    app.emit(CHARACTER_VISIBILITY_CHANGED_EVENT, visible)
}

fn resolve_explicit_visibility_change(
    current_visible: bool,
    requested_visible: bool,
) -> WindowCommand {
    match (current_visible, requested_visible) {
        (false, true) => WindowCommand::Show,
        (true, false) => WindowCommand::Hide,
        _ => WindowCommand::Noop,
    }
}

impl WindowVisibilityPolicy {
    pub fn new(_grace_polls: u8) -> Self {
        Self {
            manual_open: false,
            manual_hidden: false,
            visible: true,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn sync_visible(&mut self, visible: bool) {
        self.visible = visible;
        if visible {
            self.manual_hidden = false;
        } else {
            self.manual_open = false;
        }
    }

    pub fn mark_manual_open(&mut self) -> WindowCommand {
        self.manual_open = true;
        self.manual_hidden = false;

        if self.visible {
            WindowCommand::Noop
        } else {
            self.visible = true;
            WindowCommand::Show
        }
    }

    pub fn mark_manual_close(&mut self) -> WindowCommand {
        self.manual_open = false;
        self.manual_hidden = true;

        if self.visible {
            self.visible = false;
            WindowCommand::Hide
        } else {
            WindowCommand::Noop
        }
    }

    pub fn on_status(&mut self, _status: StatusKind) -> WindowCommand {
        // Character visibility is manual-only; status changes never show or hide the window.
        WindowCommand::Noop
    }
}

impl WindowVisibilityState {
    pub fn new(grace_polls: u8) -> Self {
        Self {
            inner: Mutex::new(WindowVisibilityPolicy::new(grace_polls)),
        }
    }

    pub fn sync_visible(&self, visible: bool) {
        self.inner
            .lock()
            .expect("window visibility state mutex poisoned")
            .sync_visible(visible);
    }

    pub fn on_status(&self, status: StatusKind) -> WindowCommand {
        self.inner
            .lock()
            .expect("window visibility state mutex poisoned")
            .on_status(status)
    }

    pub fn mark_manual_open(&self) -> WindowCommand {
        self.inner
            .lock()
            .expect("window visibility state mutex poisoned")
            .mark_manual_open()
    }

    pub fn mark_manual_close(&self) -> WindowCommand {
        self.inner
            .lock()
            .expect("window visibility state mutex poisoned")
            .mark_manual_close()
    }
}

pub fn open_settings_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    match settings_window_action(app.get_webview_window(SETTINGS_WINDOW_LABEL).is_some()) {
        SettingsWindowAction::ShowExisting => {
            if let Some(window) = app.get_webview_window(SETTINGS_WINDOW_LABEL) {
                window.show()?;
                let _ = window.set_focus();
            }
        }
        SettingsWindowAction::CreateNew => {
            let app_handle = app.clone();
            std::thread::spawn(move || {
                let builder = WebviewWindowBuilder::new(
                    &app_handle,
                    SETTINGS_WINDOW_LABEL,
                    settings_window_url(),
                )
                .title(SETTINGS_WINDOW_TITLE)
                .inner_size(640.0, 480.0)
                .resizable(true)
                .focused(true);

                if let Err(error) = builder.build() {
                    eprintln!("failed to create settings window: {error}");
                }
            });
        }
    }

    Ok(())
}

pub fn is_character_window_visible<R: Runtime>(_app: &AppHandle<R>) -> tauri::Result<bool> {
    #[cfg(windows)]
    if let Some(state) = _app.try_state::<crate::mascot_window::MascotWindowState>() {
        return Ok(state.is_visible());
    }

    Ok(false)
}

pub fn show_character_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    #[cfg(windows)]
    if let Some(state) = app.try_state::<crate::mascot_window::MascotWindowState>() {
        state.show();
        if let Some(window_state) = app.try_state::<WindowVisibilityState>() {
            window_state.sync_visible(true);
        }
        let _ = crate::tray::sync_character_toggle_menu_item(app);
        emit_character_visibility_changed(app, true)?;
        return Ok(());
    }

    if let Some(window_state) = app.try_state::<WindowVisibilityState>() {
        window_state.sync_visible(true);
    }
    let _ = crate::tray::sync_character_toggle_menu_item(app);
    emit_character_visibility_changed(app, true)?;

    Ok(())
}

pub fn hide_character_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    #[cfg(windows)]
    if let Some(state) = app.try_state::<crate::mascot_window::MascotWindowState>() {
        state.hide();
        if let Some(window_state) = app.try_state::<WindowVisibilityState>() {
            window_state.sync_visible(false);
        }
        let _ = crate::tray::sync_character_toggle_menu_item(app);
        emit_character_visibility_changed(app, false)?;
        return Ok(());
    }

    if let Some(window_state) = app.try_state::<WindowVisibilityState>() {
        window_state.sync_visible(false);
    }
    let _ = crate::tray::sync_character_toggle_menu_item(app);
    emit_character_visibility_changed(app, false)?;

    Ok(())
}

fn settings_window_action(window_exists: bool) -> SettingsWindowAction {
    if window_exists {
        SettingsWindowAction::ShowExisting
    } else {
        SettingsWindowAction::CreateNew
    }
}

fn settings_window_url() -> WebviewUrl {
    WebviewUrl::App(SETTINGS_WINDOW_ENTRY.into())
}

pub fn toggle_character_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let visible = is_character_window_visible(app)?;

    if let Some(window_state) = app.try_state::<WindowVisibilityState>() {
        window_state.sync_visible(visible);

        match if visible {
            window_state.mark_manual_close()
        } else {
            window_state.mark_manual_open()
        } {
            WindowCommand::Show => show_character_window(app)?,
            WindowCommand::Hide => hide_character_window(app)?,
            WindowCommand::Noop => {}
        }
    } else if visible {
        hide_character_window(app)?;
    } else {
        show_character_window(app)?;
    }

    Ok(())
}

#[tauri::command]
pub fn get_character_visibility(app: AppHandle) -> Result<bool, String> {
    is_character_window_visible(&app).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_character_visibility(app: AppHandle, visible: bool) -> Result<bool, String> {
    match resolve_explicit_visibility_change(
        is_character_window_visible(&app).map_err(|error| error.to_string())?,
        visible,
    ) {
        WindowCommand::Show => show_character_window(&app).map_err(|error| error.to_string())?,
        WindowCommand::Hide => hide_character_window(&app).map_err(|error| error.to_string())?,
        WindowCommand::Noop => {}
    }

    is_character_window_visible(&app).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex::StatusKind;

    #[test]
    fn window_visibility_policy_starts_visible() {
        assert!(WindowVisibilityPolicy::new(2).is_visible());
    }

    #[test]
    fn status_changes_do_not_auto_show_hidden_window() {
        let mut policy = WindowVisibilityPolicy::new(2);
        policy.sync_visible(false);

        assert_eq!(policy.on_status(StatusKind::Thinking), WindowCommand::Noop);
        assert!(!policy.is_visible());
    }

    #[test]
    fn status_changes_do_not_auto_hide_visible_window() {
        let mut policy = WindowVisibilityPolicy::new(2);

        assert_eq!(policy.on_status(StatusKind::Idle), WindowCommand::Noop);
        assert_eq!(policy.on_status(StatusKind::Idle), WindowCommand::Noop);
        assert!(policy.is_visible());
    }

    #[test]
    fn reuses_existing_settings_window_before_creating_one() {
        assert_eq!(
            settings_window_action(true),
            SettingsWindowAction::ShowExisting
        );
        assert_eq!(
            settings_window_action(false),
            SettingsWindowAction::CreateNew
        );
    }

    #[test]
    fn manual_hide_requires_manual_reopen() {
        let mut policy = WindowVisibilityPolicy::new(2);

        assert_eq!(policy.mark_manual_close(), WindowCommand::Hide);
        assert!(!policy.is_visible());
        assert_eq!(policy.on_status(StatusKind::Thinking), WindowCommand::Noop);
        assert!(!policy.is_visible());
        assert_eq!(policy.mark_manual_open(), WindowCommand::Show);
        assert!(policy.is_visible());
    }

    #[test]
    fn settings_window_uses_app_assets() {
        match settings_window_url() {
            WebviewUrl::App(path) => assert_eq!(path.to_string_lossy(), "index.html"),
            other => panic!("expected app asset URL, got {other:?}"),
        }
    }

    #[test]
    fn character_visibility_changed_event_name_is_stable() {
        assert_eq!(
            CHARACTER_VISIBILITY_CHANGED_EVENT,
            "waifudex://character-visibility-changed"
        );
    }

    #[test]
    fn resolve_explicit_visibility_change_returns_show_for_hidden_window() {
        assert_eq!(
            resolve_explicit_visibility_change(false, true),
            WindowCommand::Show
        );
    }

    #[test]
    fn resolve_explicit_visibility_change_returns_hide_for_visible_window() {
        assert_eq!(
            resolve_explicit_visibility_change(true, false),
            WindowCommand::Hide
        );
    }

    #[test]
    fn resolve_explicit_visibility_change_returns_noop_when_state_matches() {
        assert_eq!(
            resolve_explicit_visibility_change(true, true),
            WindowCommand::Noop
        );
        assert_eq!(
            resolve_explicit_visibility_change(false, false),
            WindowCommand::Noop
        );
    }
}
