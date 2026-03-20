use std::sync::Mutex;

use crate::codex::StatusKind;
use tauri::{AppHandle, Manager, Runtime};

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
            visible: true,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn sync_visible(&mut self, visible: bool) {
        self.visible = visible;
        if !visible {
            self.manual_open = false;
        }
    }

    pub fn mark_manual_open(&mut self) -> WindowCommand {
        self.idle_polls = 0;
        self.manual_open = true;

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

        if self.visible {
            self.visible = false;
            WindowCommand::Hide
        } else {
            WindowCommand::Noop
        }
    }

    pub fn on_status(&mut self, status: StatusKind) -> WindowCommand {
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
    // On Windows the native mascot window is used instead of the Tauri webview,
    // so the main window stays hidden. On other platforms, show it now that
    // setup is complete (the window starts with visible:false in tauri.conf.json
    // to avoid a brief flash of a transparent frame on startup).
    #[cfg(not(windows))]
    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
    }

    Ok(())
}

pub fn is_main_window_visible<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<bool> {
    #[cfg(windows)]
    if let Some(state) = app.try_state::<crate::mascot_window::MascotWindowState>() {
        return Ok(state.is_visible());
    }

    if let Some(window) = app.get_webview_window("main") {
        return window.is_visible();
    }

    Ok(false)
}

pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    #[cfg(windows)]
    if let Some(state) = app.try_state::<crate::mascot_window::MascotWindowState>() {
        state.show();
        let _ = crate::tray::sync_window_action_label(app);
        return Ok(());
    }

    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
        let _ = window.set_focus();
    }

    let _ = crate::tray::sync_window_action_label(app);

    Ok(())
}

pub fn hide_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    #[cfg(windows)]
    if let Some(state) = app.try_state::<crate::mascot_window::MascotWindowState>() {
        state.hide();
        let _ = crate::tray::sync_window_action_label(app);
        return Ok(());
    }

    if let Some(window) = app.get_webview_window("main") {
        window.hide()?;
    }

    let _ = crate::tray::sync_window_action_label(app);

    Ok(())
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
            WindowCommand::Noop => {
                let _ = crate::tray::sync_window_action_label(app);
            }
        }
    } else if visible {
        hide_main_window(app)?;
    } else {
        show_main_window(app)?;
    }

    Ok(())
}
