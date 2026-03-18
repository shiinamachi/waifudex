pub mod codex;
pub mod contracts;
pub mod mascot;
pub mod mascot_commands;
pub mod runtime_state;
pub mod tray;
pub mod window;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(mascot::MascotManager::new())
        .manage(runtime_state::RuntimeState::new())
        .manage(window::WindowVisibilityState::new(2))
        .invoke_handler(tauri::generate_handler![
            runtime_state::get_runtime_bootstrap,
            mascot_commands::init_mascot,
            mascot_commands::update_mascot_params,
            mascot_commands::resize_mascot,
            mascot_commands::dispose_mascot
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            window::configure_main_window(&app_handle)?;
            tray::build_tray(&app_handle)?;
            codex::start_monitor(app_handle);

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if tray::should_cleanup_on_run_event(&event) {
                tray::remove_tray_icon(app_handle);
            }
        });
}
