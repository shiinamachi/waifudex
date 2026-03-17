pub mod codex;
pub mod tray;
pub mod window;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();

            window::configure_main_window(&app_handle)?;
            tray::build_tray(&app_handle)?;
            codex::start_monitor(app_handle);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
