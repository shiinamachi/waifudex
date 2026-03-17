pub mod codex;
pub mod tray;
pub mod window;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();

            window::configure_main_window(&app_handle)?;
            tray::build_tray(&app_handle)?;
            codex::start_demo_emitter(app_handle);

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
