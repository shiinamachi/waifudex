use std::{
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
pub use waifudex_mascot::MascotParamValue;
use waifudex_mascot::MascotRenderer;

pub const MASCOT_FRAME_EVENT: &str = "waifudex://mascot-frame";

const FRAME_INTERVAL: Duration = Duration::from_millis(33);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MascotFramePayload {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub revision: u64,
}

#[derive(Debug, Default)]
pub struct MascotManager {
    inner: Mutex<Option<ActiveMascot>>,
}

#[derive(Debug)]
struct ActiveMascot {
    sender: Sender<RenderCommand>,
    thread: Option<JoinHandle<()>>,
}

#[derive(Debug)]
enum RenderCommand {
    UpdateParams(Vec<MascotParamValue>),
    Resize { width: u32, height: u32 },
    Shutdown,
}

impl MascotManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(
        &self,
        app_handle: AppHandle,
        model_path: String,
        width: u32,
        height: u32,
    ) -> Result<Vec<String>, String> {
        self.dispose()?;

        let resolved_model_path = resolve_model_path(&app_handle, &model_path)
            .ok_or_else(|| format!("mascot model path could not be resolved: {model_path}"))?;
        let (sender, receiver) = mpsc::channel();
        let (init_sender, init_receiver) = mpsc::sync_channel(1);

        let thread = thread::Builder::new()
            .name("waifudex-mascot".to_string())
            .spawn(move || render_loop(app_handle, resolved_model_path, width, height, receiver, init_sender))
            .map_err(|error| format!("failed to spawn mascot render loop: {error}"))?;

        let available_params = init_receiver
            .recv()
            .map_err(|_| "mascot render thread failed to initialize".to_string())??;

        let active = ActiveMascot {
            sender,
            thread: Some(thread),
        };

        let mut guard = self
            .inner
            .lock()
            .map_err(|_| "mascot manager mutex poisoned".to_string())?;
        *guard = Some(active);

        Ok(available_params)
    }

    pub fn update_params(&self, params: Vec<MascotParamValue>) -> Result<(), String> {
        self.with_active_mut(|active| {
            active
                .sender
                .send(RenderCommand::UpdateParams(params))
                .map_err(|_| "mascot render thread is not available".to_string())
        })
    }

    pub fn resize(&self, width: u32, height: u32) -> Result<(), String> {
        self.with_active_mut(|active| {
            active
                .sender
                .send(RenderCommand::Resize { width, height })
                .map_err(|_| "mascot render thread is not available".to_string())
        })
    }

    pub fn dispose(&self) -> Result<(), String> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| "mascot manager mutex poisoned".to_string())?;
        let active = guard.take();
        drop(guard);

        if let Some(active) = active {
            active.stop()?;
        }

        Ok(())
    }

    fn with_active_mut<T>(
        &self,
        operation: impl FnOnce(&mut ActiveMascot) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| "mascot manager mutex poisoned".to_string())?;
        let active = guard
            .as_mut()
            .ok_or_else(|| "mascot renderer is not initialized".to_string())?;
        operation(active)
    }
}

fn resolve_model_path(app_handle: &AppHandle, model_path: &str) -> Option<PathBuf> {
    let raw_path = PathBuf::from(model_path);
    if raw_path.is_absolute() && raw_path.exists() {
        return Some(raw_path);
    }

    let normalized = model_path.trim_start_matches('/');
    let resource_relative = Path::new(normalized);

    let candidate_paths: [Option<PathBuf>; 3] = [
        app_handle
            .path()
            .resource_dir()
            .ok()
            .map(|resource_dir: PathBuf| resource_dir.join(resource_relative)),
        Some(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../public")
                .join(resource_relative),
        ),
        Some(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../dist")
                .join(resource_relative),
        ),
    ];

    candidate_paths
        .into_iter()
        .flatten()
        .find(|path: &PathBuf| path.exists())
}

#[cfg(test)]
mod mascot_path_tests {
    use std::path::{Path, PathBuf};

    #[test]
    fn repo_public_models_asset_exists_for_dev_resolution() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../public/models/Aka.inx")
            .canonicalize()
            .expect("public/models/Aka.inx should exist");

        assert!(path.ends_with(PathBuf::from("public/models/Aka.inx")));
    }

    #[test]
    fn leading_slash_model_paths_normalize_to_resource_relative_segments() {
        let normalized = "/models/Aka.inx".trim_start_matches('/');
        assert_eq!(normalized, "models/Aka.inx");
    }
}

impl ActiveMascot {
    fn stop(mut self) -> Result<(), String> {
        let _ = self.sender.send(RenderCommand::Shutdown);
        if let Some(thread) = self.thread.take() {
            thread
                .join()
                .map_err(|_| "mascot render thread panicked".to_string())?;
        }
        Ok(())
    }
}

fn render_loop(
    app_handle: AppHandle,
    model_path: PathBuf,
    width: u32,
    height: u32,
    receiver: Receiver<RenderCommand>,
    init_sender: mpsc::SyncSender<Result<Vec<String>, String>>,
) {
    let mut renderer = match MascotRenderer::new(&model_path, width, height) {
        Ok(renderer) => {
            let available_params = renderer
                .available_params()
                .iter()
                .map(|param| param.name.clone())
                .collect::<Vec<_>>();
            let _ = init_sender.send(Ok(available_params));
            renderer
        }
        Err(error) => {
            let _ = init_sender.send(Err(error.to_string()));
            return;
        }
    };

    let mut running = true;

    while running {
        let started = Instant::now();

        while let Ok(command) = receiver.try_recv() {
            match command {
                RenderCommand::UpdateParams(params) => {
                    for param in &params {
                        renderer.set_param(param);
                    }
                }
                RenderCommand::Resize { width, height } => {
                    let _ = renderer.resize(width, height);
                }
                RenderCommand::Shutdown => {
                    running = false;
                    break;
                }
            }
        }

        if !running {
            break;
        }

        let payload = {
            match renderer.render_frame(FRAME_INTERVAL.as_secs_f32()) {
                Ok(Some(frame)) => {
                    let rgba = frame.to_vec();
                    let revision = renderer.revision();
                    let (width, height) = renderer.dimensions();
                    Some(MascotFramePayload {
                        width,
                        height,
                        rgba,
                        revision,
                    })
                }
                Ok(None) => None,
                Err(_) => None,
            }
        };

        if let Some(payload) = payload {
            let _ = app_handle.emit(MASCOT_FRAME_EVENT, payload);
        }

        let elapsed = started.elapsed();
        if elapsed < FRAME_INTERVAL {
            thread::sleep(FRAME_INTERVAL - elapsed);
        }
    }
}
