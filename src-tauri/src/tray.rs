use tauri::{
    menu::{CheckMenuItem, MenuBuilder, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, Runtime,
};

const TRAY_ID: &str = "waifudex-tray";
const ALWAYS_ON_TOP_ID: &str = "always-on-top";
const CHARACTER_VISIBILITY_ID: &str = "character-visibility";
const SETTINGS_ID: &str = "settings";

pub struct TraySettingsState<R: Runtime> {
    always_on_top_item: CheckMenuItem<R>,
    character_visibility_item: MenuItem<R>,
}

impl<R: Runtime> TraySettingsState<R> {
    fn new(always_on_top_item: CheckMenuItem<R>, character_visibility_item: MenuItem<R>) -> Self {
        Self {
            always_on_top_item,
            character_visibility_item,
        }
    }

    fn sync_always_on_top(&self, always_on_top: bool) -> tauri::Result<()> {
        self.always_on_top_item.set_checked(always_on_top)
    }

    fn sync_character_visibility(&self, visible: bool) -> tauri::Result<()> {
        self.character_visibility_item
            .set_text(character_toggle_label(visible))
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
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = MenuBuilder::new(app)
        .item(&character_visibility_item)
        .separator()
        .item(&settings_item)
        .item(&always_on_top_item)
        .separator()
        .item(&quit_item)
        .build()?;

    app.manage(TraySettingsState::new(
        always_on_top_item,
        character_visibility_item,
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
            "quit" => {
                remove_tray_icon(app);
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    let _ = sync_character_toggle_menu_item(app);

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

pub fn remove_tray_icon<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.remove_tray_by_id(TRAY_ID);
}

pub fn should_cleanup_on_run_event(event: &tauri::RunEvent) -> bool {
    matches!(
        event,
        tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
    )
}

fn character_toggle_label(visible: bool) -> &'static str {
    if visible {
        "Hide character"
    } else {
        "Show character"
    }
}
