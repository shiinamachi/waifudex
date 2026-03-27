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

    pub fn load_model(&mut self, model_path: &Path) -> Result<()> {
        let puppet = PuppetRenderer::new(model_path)?;
        self.puppet = puppet;
        Ok(())
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
