use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;
use tauri::{AppHandle, Manager};

const DEFAULT_MODEL_PATH: &str = "/models/Aka.inx";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelEntry {
    pub file_name: String,
    pub display_name: String,
    pub path: String,
    pub is_bundled: bool,
    pub is_active: bool,
}

fn user_models_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?
        .join("models");
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("failed to create user models dir: {e}"))?;
    }
    Ok(dir)
}

fn bundled_models_dir(app: &AppHandle) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(resource_dir) = app.path().resource_dir() {
        dirs.push(resource_dir.join("models"));
    }
    let dev_public = Path::new(env!("CARGO_MANIFEST_DIR")).join("../public/models");
    if dev_public.exists() {
        dirs.push(dev_public);
    }
    dirs
}

fn scan_inx_files(dir: &Path) -> Vec<PathBuf> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };
    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("inx"))
        })
        .collect()
}

fn active_model_path(app: &AppHandle) -> String {
    crate::app_settings::current_app_settings(app)
        .active_model_path
        .unwrap_or_else(|| DEFAULT_MODEL_PATH.to_string())
}

fn path_matches_active(file_path: &Path, active: &str) -> bool {
    let active_path = PathBuf::from(active);
    if file_path == active_path {
        return true;
    }
    if let Some(file_name) = file_path.file_name() {
        if let Some(active_name) = active_path.file_name() {
            return file_name == active_name;
        }
        let normalized = active.trim_start_matches('/');
        if let Some(name) = Path::new(normalized).file_name() {
            return file_name == name;
        }
    }
    false
}

#[tauri::command]
pub fn list_models(app: AppHandle) -> Result<Vec<ModelEntry>, String> {
    let active = active_model_path(&app);
    let mut entries = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    for dir in bundled_models_dir(&app) {
        for path in scan_inx_files(&dir) {
            let file_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if !seen_names.insert(file_name.clone()) {
                continue;
            }
            let display_name = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let is_active = path_matches_active(&path, &active);
            entries.push(ModelEntry {
                file_name: file_name.clone(),
                display_name,
                path: path.to_string_lossy().to_string(),
                is_bundled: true,
                is_active,
            });
        }
    }

    let user_dir = user_models_dir(&app)?;
    for path in scan_inx_files(&user_dir) {
        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if !seen_names.insert(file_name.clone()) {
            continue;
        }
        let display_name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let is_active = path_matches_active(&path, &active);
        entries.push(ModelEntry {
            file_name,
            display_name,
            path: path.to_string_lossy().to_string(),
            is_bundled: false,
            is_active,
        });
    }

    Ok(entries)
}

#[tauri::command]
pub fn add_model(app: AppHandle, source_path: String) -> Result<Vec<ModelEntry>, String> {
    let source = PathBuf::from(&source_path);
    if !source.exists() {
        return Err(format!("source file does not exist: {source_path}"));
    }

    let file_name = source
        .file_name()
        .ok_or_else(|| "invalid file path".to_string())?;

    let user_dir = user_models_dir(&app)?;
    let dest = user_dir.join(file_name);

    if dest.exists() {
        return Err(format!(
            "model already exists: {}",
            file_name.to_string_lossy()
        ));
    }

    fs::copy(&source, &dest).map_err(|e| format!("failed to copy model file: {e}"))?;

    list_models(app)
}

#[tauri::command]
pub fn delete_model(app: AppHandle, file_name: String) -> Result<Vec<ModelEntry>, String> {
    let user_dir = user_models_dir(&app)?;
    let target = user_dir.join(&file_name);

    if !target.exists() {
        return Err(format!("model file not found: {file_name}"));
    }

    let active = active_model_path(&app);
    if path_matches_active(&target, &active) {
        return Err("cannot delete the currently active model".to_string());
    }

    fs::remove_file(&target).map_err(|e| format!("failed to delete model file: {e}"))?;

    list_models(app)
}

#[tauri::command]
pub fn switch_model_command(app: AppHandle, model_path: String) -> Result<Vec<ModelEntry>, String> {
    crate::mascot::resolve_model_path(&app, &model_path)
        .ok_or_else(|| format!("model path could not be resolved: {model_path}"))?;

    crate::mascot::switch_model(&app, model_path.clone())?;

    crate::app_settings::update_app_settings(
        &app,
        crate::app_settings::AppSettingsUpdate {
            active_model_path: Some(model_path),
            ..Default::default()
        },
    )
    .map_err(|e| format!("failed to persist active model path: {e}"))?;

    list_models(app)
}
