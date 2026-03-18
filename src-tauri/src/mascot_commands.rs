use tauri::{AppHandle, State};

use crate::mascot::{MascotManager, MascotParamValue};

type CommandResult<T> = Result<T, String>;

#[tauri::command]
pub async fn init_mascot(
    app: AppHandle,
    state: State<'_, MascotManager>,
    model_path: String,
    width: u32,
    height: u32,
) -> CommandResult<Vec<String>> {
    state.init(app, model_path, width, height)
}

#[tauri::command]
pub async fn update_mascot_params(
    state: State<'_, MascotManager>,
    params: Vec<MascotParamValue>,
) -> CommandResult<()> {
    state.update_params(params)
}

#[tauri::command]
pub async fn resize_mascot(
    state: State<'_, MascotManager>,
    width: u32,
    height: u32,
) -> CommandResult<()> {
    state.resize(width, height)
}

#[tauri::command]
pub async fn dispose_mascot(state: State<'_, MascotManager>) -> CommandResult<()> {
    state.dispose()
}
