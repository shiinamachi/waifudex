use std::{
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use tauri::{AppHandle, Manager, Runtime};
pub use waifudex_mascot::MascotParamValue;
use waifudex_mascot::MascotRenderer;

use crate::{contracts::runtime::RuntimeStatus, mascot_motion::create_motion_targets};

const FRAME_INTERVAL: Duration = Duration::from_millis(8);

#[derive(Debug, Default)]
pub struct MascotManager {
    inner: Mutex<Option<ActiveMascot>>,
}

#[derive(Debug)]
struct ActiveMascot {
    available_params: Vec<String>,
    sender: Sender<RenderCommand>,
    thread: Option<JoinHandle<()>>,
}

#[derive(Debug)]
enum RenderCommand {
    UpdateParams(Vec<MascotParamValue>),
    SetStatus(RuntimeStatus),
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
        #[cfg(windows)]
        if std::env::var_os("WAIFUDEX_DISABLE_MASCOT_RENDERER").is_some() {
            let _ = (&self, app_handle, model_path, width, height);
            eprintln!("waifudex mascot: renderer init disabled on Windows");
            return Ok(Vec::new());
        }

        {
            let guard = self
                .inner
                .lock()
                .map_err(|_| "mascot manager mutex poisoned".to_string())?;
            if let Some(active) = guard.as_ref() {
                return Ok(active.available_params.clone());
            }
        }

        {
            let resolved_model_path = resolve_model_path(&app_handle, &model_path)
                .ok_or_else(|| format!("mascot model path could not be resolved: {model_path}"))?;
            let (sender, receiver) = mpsc::channel();
            let (init_sender, init_receiver) = mpsc::sync_channel(1);

            let thread = thread::Builder::new()
                .name("waifudex-mascot".to_string())
                .spawn(move || {
                    render_loop(
                        app_handle,
                        resolved_model_path,
                        width,
                        height,
                        receiver,
                        init_sender,
                    )
                })
                .map_err(|error| format!("failed to spawn mascot render loop: {error}"))?;

            let available_params = init_receiver
                .recv()
                .map_err(|_| "mascot render thread failed to initialize".to_string())??;

            let active = ActiveMascot {
                available_params: available_params.clone(),
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

    pub fn set_status(&self, status: RuntimeStatus) -> Result<(), String> {
        self.with_active_mut(|active| {
            active
                .sender
                .send(RenderCommand::SetStatus(status))
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

pub fn initialize_default_mascot(app_handle: &AppHandle) -> Result<Vec<String>, String> {
    let manager = app_handle.state::<MascotManager>();
    let size = app_handle
        .state::<crate::mascot_window::MascotWindowState>()
        .size();
    manager.init(
        app_handle.clone(),
        "/models/Aka.inx".to_string(),
        size.width,
        size.height,
    )
}

pub fn resize<R: Runtime>(app: &AppHandle<R>, width: u32, height: u32) -> tauri::Result<()> {
    let manager = app.state::<MascotManager>();
    manager
        .resize(width, height)
        .map_err(|e| tauri::Error::from(std::io::Error::other(e)))
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
    let started_at = Instant::now();
    let mut current_status = RuntimeStatus::Idle;
    let mut rendered_frames = 0_u64;

    while running {
        let started = Instant::now();

        while let Ok(command) = receiver.try_recv() {
            match command {
                RenderCommand::UpdateParams(params) => {
                    for param in &params {
                        renderer.set_param(param);
                    }
                }
                RenderCommand::SetStatus(status) => {
                    current_status = status;
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

        let elapsed_seconds = started_at.elapsed().as_secs_f32();
        for param in create_motion_targets(current_status, elapsed_seconds) {
            renderer.set_param(&param);
        }

        let (frame_width, frame_height) = renderer.dimensions();
        if let Ok(Some(frame)) = renderer.render_frame(FRAME_INTERVAL.as_secs_f32()) {
            rendered_frames = rendered_frames.saturating_add(1);

            #[cfg(windows)]
            {
                if rendered_frames % 60 == 0 {
                    let alpha_pixels = frame.chunks_exact(4).filter(|pixel| pixel[3] > 0).count();
                    eprintln!(
                        "waifudex mascot: rendered frame rev={} size={}x{} alpha_pixels={}",
                        rendered_frames, frame_width, frame_height, alpha_pixels
                    );
                }
            }

            let _ =
                crate::mascot_window::present_frame(&app_handle, frame_width, frame_height, frame);
        }

        let elapsed = started.elapsed();
        if elapsed < FRAME_INTERVAL {
            thread::sleep(FRAME_INTERVAL - elapsed);
        }
    }
}
