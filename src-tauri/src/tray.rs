use tauri::{
    menu::{MenuBuilder, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Wry,
};

const TRAY_ID: &str = "waifudex-tray";
const WINDOW_TOGGLE_ID: &str = "toggle-window";
const WINDOW_OPEN_LABEL: &str = "Open";
const WINDOW_CLOSE_LABEL: &str = "Close";

pub struct TrayWindowActionState {
    window_toggle_item: MenuItem<Wry>,
}

impl TrayWindowActionState {
    fn new(window_toggle_item: MenuItem<Wry>) -> Self {
        Self { window_toggle_item }
    }

    fn sync_label(&self, visible: bool) -> tauri::Result<()> {
        self.window_toggle_item
            .set_text(window_action_label(visible))
    }
}

pub fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let window_toggle_item = MenuItem::with_id(
        app,
        WINDOW_TOGGLE_ID,
        window_action_label(crate::window::is_main_window_visible(app)?),
        true,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = MenuBuilder::new(app)
        .item(&window_toggle_item)
        .separator()
        .item(&quit_item)
        .build()?;

    app.manage(TrayWindowActionState::new(window_toggle_item));

    let mut tray = TrayIconBuilder::with_id(TRAY_ID).menu(&menu);

    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }

    tray.show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            WINDOW_TOGGLE_ID => {
                let _ = crate::window::toggle_main_window(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = crate::window::toggle_main_window(&tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

pub fn sync_window_action_label(app: &AppHandle) -> tauri::Result<()> {
    let visible = crate::window::is_main_window_visible(app)?;

    if let Some(state) = app.try_state::<TrayWindowActionState>() {
        state.sync_label(visible)?;
    }

    Ok(())
}

fn window_action_label(visible: bool) -> &'static str {
    if visible {
        WINDOW_CLOSE_LABEL
    } else {
        WINDOW_OPEN_LABEL
    }
}
