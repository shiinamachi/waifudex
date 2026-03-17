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
    visible: bool,
}

impl WindowVisibilityPolicy {
    pub fn new(grace_polls: u8) -> Self {
        Self {
            grace_polls,
            idle_polls: 0,
            visible: false,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn sync_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn on_status(&mut self, status: StatusKind) -> WindowCommand {
        match status {
            StatusKind::Idle => {
                self.idle_polls = self.idle_polls.saturating_add(1);

                if self.visible && self.idle_polls >= self.grace_polls {
                    self.visible = false;
                    WindowCommand::Hide
                } else {
                    WindowCommand::Noop
                }
            }
            StatusKind::Error => {
                self.idle_polls = 0;
                if self.visible {
                    WindowCommand::Noop
                } else {
                    self.visible = true;
                    WindowCommand::Show
                }
            }
            StatusKind::Thinking
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

pub fn configure_main_window(app: &AppHandle) -> tauri::Result<()> {
    hide_main_window(app)
}

pub fn show_main_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
        let _ = window.set_focus();
    }

    Ok(())
}

pub fn hide_main_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide()?;
    }

    Ok(())
}

pub fn toggle_main_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible()? {
            hide_main_window(app)?;
        } else {
            show_main_window(app)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod window_policy_tests {
    use crate::codex::StatusKind;
    use crate::window::{WindowCommand, WindowVisibilityPolicy};

    #[test]
    fn starts_hidden_when_idle() {
        let mut policy = WindowVisibilityPolicy::new(2);

        assert!(!policy.is_visible());
        assert_eq!(policy.on_status(StatusKind::Idle), WindowCommand::Noop);
    }

    #[test]
    fn shows_on_first_active_status() {
        let mut policy = WindowVisibilityPolicy::new(2);

        assert_eq!(policy.on_status(StatusKind::Thinking), WindowCommand::Show);
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
}
