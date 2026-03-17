use tauri::{AppHandle, Manager};

pub fn configure_main_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
        let _ = window.set_focus();
    }

    Ok(())
}

pub fn toggle_main_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible()? {
            window.hide()?;
        } else {
            window.show()?;
            let _ = window.set_focus();
        }
    }

    Ok(())
}
