use std::sync::Mutex;

use crate::codex::StatusKind;
use tauri::{AppHandle, Manager};

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
            StatusKind::Error
            | StatusKind::CodexNotInstalled
            | StatusKind::Thinking
            | StatusKind::Writing
            | StatusKind::RunningTests
            | StatusKind::Success => {
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

pub fn configure_main_window(app: &AppHandle) -> tauri::Result<()> {
    let _ = app;
    Ok(())
}

pub fn is_main_window_visible(app: &AppHandle) -> tauri::Result<bool> {
    if let Some(window) = app.get_webview_window("main") {
        return window.is_visible();
    }

    Ok(false)
}

pub fn show_main_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
        let _ = window.set_focus();
    }

    let _ = crate::tray::sync_window_action_label(app);

    Ok(())
}

pub fn hide_main_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide()?;
    }

    let _ = crate::tray::sync_window_action_label(app);

    Ok(())
}

pub fn toggle_main_window(app: &AppHandle) -> tauri::Result<()> {
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

#[cfg(test)]
mod window_policy_tests {
    use crate::codex::StatusKind;
    use crate::window::{WindowCommand, WindowVisibilityPolicy};

    #[test]
    fn starts_visible_on_app_launch() {
        let mut policy = WindowVisibilityPolicy::new(2);

        assert!(policy.is_visible());
        assert_eq!(policy.on_status(StatusKind::Idle), WindowCommand::Noop);
    }

    #[test]
    fn shows_on_first_active_status() {
        let mut policy = WindowVisibilityPolicy::new(2);

        assert_eq!(policy.on_status(StatusKind::Thinking), WindowCommand::Noop);
        assert!(policy.is_visible());
    }

    #[test]
    fn keeps_the_window_visible_on_error() {
        let mut policy = WindowVisibilityPolicy::new(2);
        let _ = policy.on_status(StatusKind::Thinking);

        assert_eq!(policy.on_status(StatusKind::Error), WindowCommand::Noop);
        assert!(policy.is_visible());
    }

    #[test]
    fn keeps_the_window_visible_when_codex_is_not_installed() {
        let mut policy = WindowVisibilityPolicy::new(2);
        let _ = policy.on_status(StatusKind::Thinking);

        assert_eq!(
            policy.on_status(StatusKind::CodexNotInstalled),
            WindowCommand::Noop
        );
        assert!(policy.is_visible());
    }

    #[test]
    fn hides_after_idle_grace_period_expires() {
        let mut policy = WindowVisibilityPolicy::new(2);
        let _ = policy.on_status(StatusKind::Writing);

        assert_eq!(policy.on_status(StatusKind::Idle), WindowCommand::Noop);
        assert!(policy.is_visible());

        assert_eq!(policy.on_status(StatusKind::Idle), WindowCommand::Hide);
        assert!(!policy.is_visible());
    }

    #[test]
    fn shows_again_if_window_was_manually_hidden_while_codex_is_active() {
        let mut policy = WindowVisibilityPolicy::new(2);
        let _ = policy.on_status(StatusKind::Thinking);

        policy.sync_visible(false);

        assert_eq!(policy.on_status(StatusKind::Writing), WindowCommand::Show);
        assert!(policy.is_visible());
    }

    #[test]
    fn keeps_window_visible_during_idle_when_user_opened_it() {
        let mut policy = WindowVisibilityPolicy::new(2);
        policy.sync_visible(false);

        assert_eq!(policy.mark_manual_open(), WindowCommand::Show);
        assert!(policy.is_visible());

        assert_eq!(policy.on_status(StatusKind::Idle), WindowCommand::Noop);
        assert!(policy.is_visible());

        assert_eq!(policy.on_status(StatusKind::Idle), WindowCommand::Noop);
        assert!(policy.is_visible());
    }

    #[test]
    fn manual_close_clears_manual_open_override() {
        let mut policy = WindowVisibilityPolicy::new(2);

        assert_eq!(policy.mark_manual_open(), WindowCommand::Noop);
        assert!(policy.is_visible());

        assert_eq!(policy.mark_manual_close(), WindowCommand::Hide);
        assert!(!policy.is_visible());
        assert_eq!(policy.on_status(StatusKind::Idle), WindowCommand::Noop);
    }
}
