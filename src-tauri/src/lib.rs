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

    fn parse_json(source: &str) -> Value {
        serde_json::from_str(source).expect("json should parse")
    }

    fn tauri_config() -> Value {
        parse_json(include_str!("../tauri.conf.json"))
    }

    fn tauri_windows_build_config() -> Value {
        parse_json(include_str!("../tauri.windows.build.conf.json"))
    }

    fn tauri_linux_config() -> Value {
        parse_json(include_str!("../tauri.linux.conf.json"))
    }

    fn tauri_windows_updater_config() -> Value {
        parse_json(include_str!("../tauri.windows.updater.conf.json"))
    }

    fn tauri_windows_platform_config() -> Value {
        parse_json(include_str!("../tauri.windows.conf.json"))
    }

    fn apply_merge_patch(target: &mut Value, patch: &Value) {
        match (target, patch) {
            (Value::Object(target_obj), Value::Object(patch_obj)) => {
                for (key, patch_value) in patch_obj {
                    if patch_value.is_null() {
                        target_obj.remove(key);
                        continue;
                    }

                    match target_obj.get_mut(key) {
                        Some(target_value) => apply_merge_patch(target_value, patch_value),
                        None => {
                            target_obj.insert(key.clone(), patch_value.clone());
                        }
                    }
                }
            }
            (target_value, patch_value) => {
                *target_value = patch_value.clone();
            }
        }
    }

    fn merged_config(overlay: Value) -> Value {
        let mut merged = tauri_config();
        apply_merge_patch(&mut merged, &overlay);
        merged
    }

    fn tauri_config_version() -> String {
        tauri_config()["version"]
            .as_str()
            .expect("tauri config version should be a string")
            .to_owned()
    }

    fn package_json_version() -> String {
        let package = parse_json(include_str!("../../package.json"));
        package["version"]
            .as_str()
            .expect("package.json version should be a string")
            .to_owned()
    }

    fn assert_windows_bundle_resources(config: &Value, config_name: &str) {
        let resources = config["bundle"]["resources"]
            .as_array()
            .unwrap_or_else(|| panic!("{config_name} should define bundle.resources"));

        assert!(
            resources
                .iter()
                .filter_map(Value::as_str)
                .any(|resource| resource.ends_with("/inochi2d-c.dll")),
            "{config_name} should bundle the Windows inochi2d DLL"
        );
        assert!(
            resources
                .iter()
                .filter_map(Value::as_str)
                .all(|resource| !resource.ends_with("/libinochi2d-c.so")),
            "{config_name} should not bundle the Linux inochi2d shared object"
        );
    }

    fn assert_linux_bundle_resources(config: &Value, config_name: &str) {
        let resources = config["bundle"]["resources"]
            .as_array()
            .unwrap_or_else(|| panic!("{config_name} should define bundle.resources"));

        assert!(
            resources
                .iter()
                .filter_map(Value::as_str)
                .any(|resource| resource.ends_with("/libinochi2d-c.so")),
            "{config_name} should bundle the Linux inochi2d shared object"
        );
        assert!(
            resources
                .iter()
                .filter_map(Value::as_str)
                .all(|resource| !resource.ends_with("/inochi2d-c.dll")),
            "{config_name} should not bundle the Windows inochi2d DLL"
        );
    }

    #[test]
    fn shared_config_does_not_bundle_platform_specific_runtime() {
        assert!(
            tauri_config()["bundle"]["resources"].is_null(),
            "tauri.conf.json should not define platform-specific bundle.resources"
        );
    }

    #[test]
    fn app_version_matches_all_config_sources() {
        let app_version = env!("CARGO_PKG_VERSION");

        assert_eq!(app_version, tauri_config_version());
        assert_eq!(app_version, package_json_version());
    }

    #[test]
    fn windows_build_config_bundles_windows_inochi2d_runtime() {
        assert_windows_bundle_resources(
            &merged_config(tauri_windows_build_config()),
            "tauri.windows.build.conf.json",
        );
    }

    #[test]
    fn windows_updater_config_bundles_windows_inochi2d_runtime() {
        assert_windows_bundle_resources(
            &merged_config(tauri_windows_updater_config()),
            "tauri.windows.updater.conf.json",
        );
    }

    #[test]
    fn windows_platform_config_bundles_windows_inochi2d_runtime() {
        assert_windows_bundle_resources(
            &merged_config(tauri_windows_platform_config()),
            "tauri.windows.conf.json",
        );
    }

    #[test]
    fn linux_config_bundles_linux_inochi2d_runtime() {
        assert_linux_bundle_resources(
            &merged_config(tauri_linux_config()),
            "tauri.linux.conf.json",
        );
    }
}
