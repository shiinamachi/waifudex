use tauri::{
    menu::{CheckMenuItem, MenuBuilder, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, Runtime,
};

const TRAY_ID: &str = "waifudex-tray";
const ALWAYS_ON_TOP_ID: &str = "always-on-top";
const CHARACTER_VISIBILITY_ID: &str = "character-visibility";
const RESTART_UPDATE_ID: &str = "restart-update";
const SETTINGS_ID: &str = "settings";

pub struct TraySettingsState<R: Runtime> {
    always_on_top_item: CheckMenuItem<R>,
    character_visibility_item: MenuItem<R>,
    restart_update_item: MenuItem<R>,
}

impl<R: Runtime> TraySettingsState<R> {
    fn new(
        always_on_top_item: CheckMenuItem<R>,
        character_visibility_item: MenuItem<R>,
        restart_update_item: MenuItem<R>,
    ) -> Self {
        Self {
            always_on_top_item,
            character_visibility_item,
            restart_update_item,
        }
    }

    fn sync_always_on_top(&self, always_on_top: bool) -> tauri::Result<()> {
        self.always_on_top_item.set_checked(always_on_top)
    }

    fn sync_character_visibility(&self, visible: bool) -> tauri::Result<()> {
        self.character_visibility_item
            .set_text(character_toggle_label(visible))
    }

    fn sync_restart_update(&self, ready: bool) -> tauri::Result<()> {
        self.restart_update_item
            .set_text(update_restart_label(ready))?;
        self.restart_update_item
            .set_enabled(should_enable_update_restart_item(ready))
    }
}

pub fn build_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let current_settings = crate::app_settings::current_app_settings(app);
    let character_visibility_item = MenuItem::with_id(
        app,
        CHARACTER_VISIBILITY_ID,
        character_toggle_label(crate::window::is_character_window_visible(app).unwrap_or(true)),
        true,
        None::<&str>,
    )?;
    let always_on_top_item = CheckMenuItem::with_id(
        app,
        ALWAYS_ON_TOP_ID,
        "Always on Top",
        true,
        current_settings.always_on_top,
        None::<&str>,
    )?;
    let settings_item = MenuItem::with_id(app, SETTINGS_ID, "Settings", true, None::<&str>)?;
    let restart_update_item = MenuItem::with_id(
        app,
        RESTART_UPDATE_ID,
        update_restart_label(crate::app_update::is_update_ready(app)),
        should_enable_update_restart_item(crate::app_update::is_update_ready(app)),
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = MenuBuilder::new(app)
        .item(&character_visibility_item)
        .separator()
        .item(&restart_update_item)
        .item(&settings_item)
        .item(&always_on_top_item)
        .separator()
        .item(&quit_item)
        .build()?;

    app.manage(TraySettingsState::new(
        always_on_top_item,
        character_visibility_item,
        restart_update_item,
    ));

    let mut tray = TrayIconBuilder::with_id(TRAY_ID).menu(&menu);

    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }

    tray.show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            CHARACTER_VISIBILITY_ID => {
                if let Err(error) = crate::window::toggle_character_window(app) {
                    eprintln!("failed to toggle character visibility: {error}");
                }

                let _ = sync_character_toggle_menu_item(app);
            }
            ALWAYS_ON_TOP_ID => {
                let current_settings = crate::app_settings::current_app_settings(app);
                let update = crate::app_settings::AppSettingsUpdate {
                    always_on_top: Some(!current_settings.always_on_top),
                    ..Default::default()
                };

                if let Err(error) = crate::app_settings::update_app_settings(app, update) {
                    eprintln!("failed to update always-on-top setting: {error}");
                }

                let _ = sync_always_on_top_menu_item(app);
            }
            SETTINGS_ID => {
                let _ = crate::window::open_settings_window(app);
            }
            RESTART_UPDATE_ID => {
                let _ = crate::app_update::restart_to_apply_update(app.clone());
            }
            "quit" => {
                remove_tray_icon(app);
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    let _ = sync_character_toggle_menu_item(app);
    let _ = sync_update_restart_menu_item(app);

    Ok(())
}

pub fn sync_always_on_top_menu_item<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(state) = app.try_state::<TraySettingsState<R>>() {
        state.sync_always_on_top(crate::app_settings::current_app_settings(app).always_on_top)?;
    }

    Ok(())
}

pub fn sync_character_toggle_menu_item<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(state) = app.try_state::<TraySettingsState<R>>() {
        state.sync_character_visibility(crate::window::is_character_window_visible(app)?)?;
    }

    Ok(())
}

pub fn sync_update_restart_menu_item<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(state) = app.try_state::<TraySettingsState<R>>() {
        let ready = app
            .try_state::<crate::app_update::AppUpdateState>()
            .map(|update_state| {
                update_state.snapshot().status == crate::app_update::AppUpdateStatus::ReadyToRestart
            })
            .unwrap_or(false);
        state.sync_restart_update(ready)?;
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
    true
}

#[cfg_attr(not(test), allow(dead_code))]
fn should_toggle_window_on_left_click() -> bool {
    false
}

fn character_toggle_label(visible: bool) -> &'static str {
    if visible {
        "Hide character"
    } else {
        "Show character"
    }
}

fn update_restart_label(ready: bool) -> &'static str {
    if ready {
        "Restart to update"
    } else {
        "No update ready"
    }
}

fn should_enable_update_restart_item(ready: bool) -> bool {
    ready
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_policy_includes_always_on_top_toggle() {
        assert!(should_include_always_on_top_menu_item());
    }

    #[test]
    fn tray_policy_includes_window_toggle_menu_item() {
        assert!(should_include_window_toggle_menu_item());
    }

    #[test]
    fn hidden_character_uses_show_label() {
        assert_eq!(character_toggle_label(false), "Show character");
    }

    #[test]
    fn visible_character_uses_hide_label() {
        assert_eq!(character_toggle_label(true), "Hide character");
    }

    #[test]
    fn tray_policy_disables_left_click_window_toggle() {
        assert!(!should_toggle_window_on_left_click());
    }

    #[test]
    fn updater_restart_label_only_uses_restart_text_when_update_is_ready() {
        assert_eq!(update_restart_label(true), "Restart to update");
        assert_ne!(update_restart_label(false), "Restart to update");
    }

    #[test]
    fn updater_restart_item_is_disabled_without_a_pending_update() {
        assert!(should_enable_update_restart_item(true));
        assert!(!should_enable_update_restart_item(false));
    }
}
