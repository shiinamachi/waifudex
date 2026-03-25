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
    use std::{fs, path::PathBuf};

    use serde_json::{json, Value};

    fn src_tauri_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn read_json_config(path: &str) -> Value {
        let content = fs::read_to_string(src_tauri_root().join(path))
            .unwrap_or_else(|error| panic!("failed to read {path}: {error}"));
        serde_json::from_str(&content)
            .unwrap_or_else(|error| panic!("failed to parse {path}: {error}"))
    }

    fn merge_json(target: &mut Value, source: Value) {
        match (target, source) {
            (Value::Object(target_map), Value::Object(source_map)) => {
                for (key, value) in source_map {
                    merge_json(target_map.entry(key).or_insert(Value::Null), value);
                }
            }
            (target_slot, source_value) => {
                *target_slot = source_value;
            }
        }
    }

    fn merged_windows_bundle_config() -> Value {
        let mut base = read_json_config("tauri.conf.json");
        let windows = read_json_config("tauri.windows.conf.json");
        merge_json(&mut base, windows);
        base["bundle"].clone()
    }

    fn read_windows_installer_script() -> String {
        fs::read_to_string(src_tauri_root().join("windows/installer.nsi"))
            .unwrap_or_else(|error| panic!("failed to read windows/installer.nsi: {error}"))
    }

    #[test]
    fn windows_nsis_bundle_contract_uses_custom_template() {
        let bundle = merged_windows_bundle_config();
        let nsis = &bundle["windows"]["nsis"];

        assert_eq!(bundle["targets"], json!(["nsis"]));
        assert_eq!(nsis["template"].as_str(), Some("windows/installer.nsi"));
        assert_eq!(nsis["installMode"].as_str(), Some("currentUser"));
        assert_eq!(nsis["installerIcon"].as_str(), Some("icons/icon.ico"));
    }

    #[test]
    fn windows_nsis_installer_script_contract_matches_squirrel_style_ui() {
        let installer_script = read_windows_installer_script();

        for expected in [
            r#"!define MUI_UI "${NSISDIR}\Contrib\UIs\sdbarker_tiny.exe""#,
            r#"Caption "${PRODUCTNAME}""#,
            r#"BrandingText " ""#,
            r#"ShowInstDetails nevershow"#,
            r#"ShowUninstDetails nevershow"#,
            r#"!insertmacro MUI_PAGE_STARTMENU Application $AppStartMenuFolder"#,
        ] {
            assert!(
                installer_script.contains(expected),
                "missing installer contract string: {expected}",
            );
        }
    }

    #[test]
    fn windows_nsis_uninstaller_progress_keeps_close_button_available() {
        let installer_script = read_windows_installer_script();
        let offending_snippet = r#"Function un.ProgressShow
  GetDlgItem $0 $HWNDPARENT 1
  ShowWindow $0 0"#;

        assert!(
            !installer_script.contains(offending_snippet),
            "uninstaller progress must not hide the Close button",
        );
    }
}
