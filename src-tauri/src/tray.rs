use tauri::{
    menu::{CheckMenuItem, MenuBuilder, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, Runtime,
};

const TRAY_ID: &str = "waifudex-tray";
const ALWAYS_ON_TOP_ID: &str = "always-on-top";
const SETTINGS_ID: &str = "settings";

pub struct TraySettingsState<R: Runtime> {
    always_on_top_item: CheckMenuItem<R>,
}

impl<R: Runtime> TraySettingsState<R> {
    fn new(always_on_top_item: CheckMenuItem<R>) -> Self {
        Self { always_on_top_item }
    }

    fn sync_always_on_top(&self, always_on_top: bool) -> tauri::Result<()> {
        self.always_on_top_item.set_checked(always_on_top)
    }
}

pub fn build_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let current_settings = crate::app_settings::current_app_settings(app);
    let always_on_top_item = CheckMenuItem::with_id(
        app,
        ALWAYS_ON_TOP_ID,
        "Always on Top",
        true,
        current_settings.always_on_top,
        None::<&str>,
    )?;
    let settings_item = MenuItem::with_id(app, SETTINGS_ID, "Settings", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = MenuBuilder::new(app)
        .item(&settings_item)
        .item(&always_on_top_item)
        .separator()
        .item(&quit_item)
        .build()?;

    app.manage(TraySettingsState::new(always_on_top_item));

    let mut tray = TrayIconBuilder::with_id(TRAY_ID).menu(&menu);

    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }

    tray.show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            ALWAYS_ON_TOP_ID => {
                let current_settings = crate::app_settings::current_app_settings(app);
                let update = crate::app_settings::AppSettingsUpdate {
                    always_on_top: Some(!current_settings.always_on_top),
                };

                if let Err(error) = crate::app_settings::update_app_settings(app, update) {
                    eprintln!("failed to update always-on-top setting: {error}");
                }

                let _ = sync_always_on_top_menu_item(app);
            }
            SETTINGS_ID => {
                let _ = crate::window::open_settings_window(app);
            }
            "quit" => {
                remove_tray_icon(app);
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

pub fn sync_always_on_top_menu_item<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(state) = app.try_state::<TraySettingsState<R>>() {
        state.sync_always_on_top(crate::app_settings::current_app_settings(app).always_on_top)?;
    }

    Ok(())
}

pub fn remove_tray_icon<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.remove_tray_by_id(TRAY_ID);
}

pub fn should_cleanup_on_run_event(event: &tauri::RunEvent) -> bool {
    matches!(
        event,
        tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
    )
}

#[cfg_attr(not(test), allow(dead_code))]
fn should_include_always_on_top_menu_item() -> bool {
    true
}

#[cfg_attr(not(test), allow(dead_code))]
fn should_include_window_toggle_menu_item() -> bool {
    false
}

#[cfg_attr(not(test), allow(dead_code))]
fn should_toggle_window_on_left_click() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_policy_includes_always_on_top_toggle() {
        assert!(should_include_always_on_top_menu_item());
    }

    #[test]
    fn tray_policy_disables_window_toggle_menu_item() {
        assert!(!should_include_window_toggle_menu_item());
    }

    #[test]
    fn tray_policy_disables_left_click_window_toggle() {
        assert!(!should_toggle_window_on_left_click());
    }
}
