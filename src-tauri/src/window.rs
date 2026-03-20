use std::sync::Mutex;

use crate::codex::StatusKind;
use tauri::{AppHandle, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

const MAIN_WINDOW_LABEL: &str = "main";
const SETTINGS_WINDOW_LABEL: &str = "settings";
const SETTINGS_WINDOW_TITLE: &str = "Settings";

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
    grace_polls: u8,
    idle_polls: u8,
    manual_open: bool,
    manual_hidden: bool,
    visible: bool,
}

pub struct WindowVisibilityState {
    inner: Mutex<WindowVisibilityPolicy>,
}

impl WindowVisibilityPolicy {
    pub fn new(grace_polls: u8) -> Self {
        Self {
            grace_polls,
            idle_polls: 0,
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
        self.idle_polls = 0;
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
        self.idle_polls = 0;
        self.manual_open = false;
        self.manual_hidden = true;

        if self.visible {
            self.visible = false;
            WindowCommand::Hide
        } else {
            WindowCommand::Noop
        }
    }

    pub fn on_status(&mut self, status: StatusKind) -> WindowCommand {
        if self.manual_hidden {
            self.idle_polls = 0;

            if self.visible {
                self.visible = false;
                return WindowCommand::Hide;
            }

            return WindowCommand::Noop;
        }

        match status {
            StatusKind::Idle => {
                if self.manual_open {
                    self.idle_polls = 0;
                    return WindowCommand::Noop;
                }

                self.idle_polls = self.idle_polls.saturating_add(1);

                if self.visible && self.idle_polls >= self.grace_polls {
                    self.visible = false;
                    WindowCommand::Hide
                } else {
                    WindowCommand::Noop
                }
            }
            StatusKind::CodexNotInstalled
            | StatusKind::Thinking
            | StatusKind::Coding
            | StatusKind::Question
            | StatusKind::Complete => {
                self.idle_polls = 0;
                if self.visible {
                    WindowCommand::Noop
                } else {
                    self.visible = true;
                    WindowCommand::Show
                }
            }
        }
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

pub fn configure_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if should_show_main_window_on_setup() {
        show_main_window(app)?;
    }

    Ok(())
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
                    WebviewUrl::External(
                        "about:blank"
                            .parse()
                            .expect("hardcoded about:blank URL must parse"),
                    ),
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

pub fn is_main_window_visible<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<bool> {
    #[cfg(windows)]
    if let Some(state) = app.try_state::<crate::mascot_window::MascotWindowState>() {
        return Ok(state.is_visible());
    }

    if should_use_main_webview() {
        if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
            return window.is_visible();
        }
    }

    Ok(false)
}

pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    #[cfg(windows)]
    if let Some(state) = app.try_state::<crate::mascot_window::MascotWindowState>() {
        state.show();
        if let Some(window_state) = app.try_state::<WindowVisibilityState>() {
            window_state.sync_visible(true);
        }
        let _ = crate::tray::sync_character_toggle_menu_item(app);
        return Ok(());
    }

    if should_use_main_webview() {
        if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
            window.show()?;
            let _ = window.set_focus();
        }
    }

    if let Some(window_state) = app.try_state::<WindowVisibilityState>() {
        window_state.sync_visible(true);
    }
    let _ = crate::tray::sync_character_toggle_menu_item(app);

    Ok(())
}

pub fn hide_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    #[cfg(windows)]
    if let Some(state) = app.try_state::<crate::mascot_window::MascotWindowState>() {
        state.hide();
        if let Some(window_state) = app.try_state::<WindowVisibilityState>() {
            window_state.sync_visible(false);
        }
        let _ = crate::tray::sync_character_toggle_menu_item(app);
        return Ok(());
    }

    if should_use_main_webview() {
        if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
            window.hide()?;
        }
    }

    if let Some(window_state) = app.try_state::<WindowVisibilityState>() {
        window_state.sync_visible(false);
    }
    let _ = crate::tray::sync_character_toggle_menu_item(app);

    Ok(())
}

fn should_show_main_window_on_setup() -> bool {
    true
}

fn should_use_main_webview() -> bool {
    false
}

fn settings_window_action(window_exists: bool) -> SettingsWindowAction {
    if window_exists {
        SettingsWindowAction::ShowExisting
    } else {
        SettingsWindowAction::CreateNew
    }
}

pub fn toggle_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let visible = is_main_window_visible(app)?;

    if let Some(window_state) = app.try_state::<WindowVisibilityState>() {
        window_state.sync_visible(visible);

        match if visible {
            window_state.mark_manual_close()
        } else {
            window_state.mark_manual_open()
        } {
            WindowCommand::Show => show_main_window(app)?,
            WindowCommand::Hide => hide_main_window(app)?,
            WindowCommand::Noop => {}
        }
    } else if visible {
        hide_main_window(app)?;
    } else {
        show_main_window(app)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex::StatusKind;

    #[test]
    fn main_window_is_shown_during_setup() {
        assert!(should_show_main_window_on_setup());
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
    fn manual_hide_blocks_automatic_show_until_manual_open() {
        let mut policy = WindowVisibilityPolicy::new(2);

        assert_eq!(policy.mark_manual_close(), WindowCommand::Hide);
        assert!(!policy.is_visible());
        assert_eq!(policy.on_status(StatusKind::Thinking), WindowCommand::Noop);
        assert!(!policy.is_visible());
        assert_eq!(policy.mark_manual_open(), WindowCommand::Show);
        assert!(policy.is_visible());
    }
}
