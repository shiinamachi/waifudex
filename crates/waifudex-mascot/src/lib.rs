mod context;
mod frame;
#[cfg(feature = "native-inochi2d")]
mod native;
mod puppet;

use std::{path::PathBuf, result::Result as StdResult};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(not(feature = "native-inochi2d"))]
use std::path::Path;

#[cfg(not(feature = "native-inochi2d"))]
use context::RendererContext;
#[cfg(not(feature = "native-inochi2d"))]
use frame::FrameBuffer;
#[cfg(not(feature = "native-inochi2d"))]
use puppet::PuppetRenderer;

pub type Result<T> = StdResult<T, MascotError>;

#[derive(Debug, Error)]
pub enum MascotError {
    #[error("mascot model was not found: {0}")]
    ModelNotFound(PathBuf),
    #[error("failed to initialize native OpenGL backend: {0}")]
    NativeContext(String),
    #[error("inochi2d native call failed: {0}")]
    NativeFfi(String),
    #[error("native inochi2d backend is not available in this build")]
    NativeBackendUnavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MascotParamValue {
    pub name: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParamInfo {
    pub name: String,
    pub is_vec2: bool,
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub defaults: [f32; 2],
}

#[cfg(not(feature = "native-inochi2d"))]
#[derive(Debug)]
pub struct MascotRenderer {
    context: RendererContext,
    frame: FrameBuffer,
    puppet: PuppetRenderer,
}

#[cfg(not(feature = "native-inochi2d"))]
impl MascotRenderer {
    pub fn new(model_path: &Path, width: u32, height: u32) -> Result<Self> {
        let context = RendererContext::new(width, height);
        let (width, height) = context.dimensions();
        let frame = FrameBuffer::new(width, height);
        let puppet = PuppetRenderer::new(model_path)?;

        Ok(Self {
            context,
            frame,
            puppet,
        })
    }

    pub fn set_param(&mut self, param: &MascotParamValue) -> bool {
        self.puppet.set_param(param)
    }

    pub fn render_frame(&mut self, dt: f32) -> Result<Option<&[u8]>> {
        if self.puppet.render_if_needed(&mut self.frame, dt) {
            Ok(Some(self.frame.pixels()))
        } else {
            Ok(None)
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.context.resize(width, height);
        let (width, height) = self.context.dimensions();
        self.frame.resize(width, height);
        self.puppet.mark_dirty();
        Ok(())
    }

    pub fn available_params(&self) -> &[ParamInfo] {
        self.puppet.available_params()
    }

    pub fn dimensions(&self) -> (u32, u32) {
        self.context.dimensions()
    }

    pub fn revision(&self) -> u64 {
        self.frame.revision()
    }
}

#[cfg(feature = "native-inochi2d")]
pub use native::NativeMascotRenderer as MascotRenderer;

#[cfg(test)]
mod tests {
    use super::MascotRenderer;
    use std::{
        fs,
        path::PathBuf,
        sync::{Mutex, OnceLock},
    };

    fn fixture_model_path() -> PathBuf {
        let path = std::env::temp_dir().join("waifudex-mascot-test.inx");
        if !path.exists() {
            fs::write(&path, b"stub").expect("write fixture model");
        }
        path
    }

    #[cfg(feature = "native-inochi2d")]
    fn real_model_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../public/models/Aka.inx")
            .canonicalize()
            .expect("real model path")
    }

    #[cfg(feature = "native-inochi2d")]
    fn native_test_guard() -> std::sync::MutexGuard<'static, ()> {
        static NATIVE_TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
        NATIVE_TEST_MUTEX
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("native test mutex")
    }

    #[cfg(not(feature = "native-inochi2d"))]
    #[test]
    fn renderer_reports_available_params_and_pixels() {
        let mut renderer = MascotRenderer::new(&fixture_model_path(), 32, 32).expect("renderer");

        assert!(!renderer.available_params().is_empty());

        let pixels = renderer
            .render_frame(1.0 / 60.0)
            .expect("render")
            .expect("initial frame");
        assert_eq!(pixels.len(), 32 * 32 * 4);
        assert!(pixels.iter().any(|channel| *channel > 0));
    }

    #[cfg(not(feature = "native-inochi2d"))]
    #[test]
    fn renderer_marks_changes_when_params_update() {
        use super::MascotParamValue;

        let mut renderer = MascotRenderer::new(&fixture_model_path(), 24, 24).expect("renderer");
        let _ = renderer.render_frame(1.0 / 60.0).expect("initial render");

        assert!(renderer.set_param(&MascotParamValue {
            name: "ParamMouthSmile".to_string(),
            x: 0.0,
            y: 1.0,
        }));
        assert!(renderer
            .render_frame(1.0 / 60.0)
            .expect("second render")
            .is_some());
    }

    #[cfg(feature = "native-inochi2d")]
    #[test]
    fn native_renderer_loads_real_model_discovers_params_and_emits_a_frame() {
        let _guard = native_test_guard();
        let mut renderer =
            MascotRenderer::new(&real_model_path(), 128, 128).expect("native renderer");

        assert!(!renderer.available_params().is_empty());
        assert!(renderer
            .available_params()
            .iter()
            .all(|param| !param.name.is_empty()));

        let frame = renderer
            .render_frame(1.0 / 60.0)
            .expect("render frame")
            .expect("first frame");

        assert!(frame.iter().any(|channel| *channel != 0));

        let mut min_x = 128usize;
        let mut min_y = 128usize;
        let mut max_x = 0usize;
        let mut max_y = 0usize;
        let mut count = 0usize;
        for y in 0..128usize {
            for x in 0..128usize {
                let alpha = frame[(y * 128 + x) * 4 + 3];
                if alpha > 0 {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                    count += 1;
                }
            }
        }
        eprintln!(
            "native bbox x={}..{} y={}..{} count={}",
            min_x, max_x, min_y, max_y, count
        );
    }
}
