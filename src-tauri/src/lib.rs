use tauri::Manager;

pub mod app_settings;
pub mod app_update;
pub mod codex;
pub mod contracts;
pub mod external_link;
pub mod mascot;
pub mod mascot_motion;
pub mod mascot_window;
pub mod runtime_state;
pub mod tray;
pub mod window;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .on_window_event(|window, event| {
            if window.label() == crate::window::SETTINGS_WINDOW_LABEL {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .manage(app_settings::AppSettingsState::new())
        .manage(mascot::MascotManager::new())
        .manage(mascot_window::MascotWindowState::new())
        .manage(runtime_state::RuntimeState::new())
        .manage(window::WindowVisibilityState::new(2))
        .invoke_handler(tauri::generate_handler![
            runtime_state::get_runtime_bootstrap,
            app_update::get_app_update_state,
            app_update::check_for_updates_command,
            app_update::restart_to_apply_update_command,
            app_settings::get_app_settings,
            app_settings::update_app_settings_command,
            window::get_character_visibility,
            window::set_character_visibility,
            mascot_window::get_display_monitors,
            mascot_window::move_character_window_command
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            app.manage(app_update::AppUpdateState::new(
                app.package_info().version.to_string(),
            ));
            external_link::register_open_external_url_listener(&app_handle);
            app_settings::initialize(&app_handle)?;
            mascot_window::initialize(&app_handle)?;
            let _ = app_settings::sync_display_monitor_on_startup(&app_handle);
            let _ = mascot::initialize_default_mascot(&app_handle);
            window::show_character_window(&app_handle)?;
            tray::build_tray(&app_handle)?;
            app_update::start_startup_check(app_handle.clone());
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

#[cfg(test)]
mod tests {
    use serde_json::Value;

    fn tauri_config_version() -> String {
        let config: Value = serde_json::from_str(include_str!("../tauri.conf.json"))
            .expect("tauri.conf.json should parse");
        config["version"]
            .as_str()
            .expect("tauri config version should be a string")
            .to_owned()
    }

    fn package_json_version() -> String {
        let package: Value = serde_json::from_str(include_str!("../../package.json"))
            .expect("package.json should parse");
        package["version"]
            .as_str()
            .expect("package.json version should be a string")
            .to_owned()
    }

    #[test]
    fn app_version_matches_all_config_sources() {
        let app_version = env!("CARGO_PKG_VERSION");

        assert_eq!(app_version, tauri_config_version());
        assert_eq!(app_version, package_json_version());
    }
}
