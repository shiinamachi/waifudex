pub mod param_bridge;
#[cfg(target_arch = "wasm32")]
pub mod renderer;

#[cfg(target_arch = "wasm32")]
use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
use glam::Vec2;
#[cfg(target_arch = "wasm32")]
use inox2d::formats::inp::parse_inp;
#[cfg(target_arch = "wasm32")]
use inox2d::puppet::Puppet;
#[cfg(target_arch = "wasm32")]
use inox2d::render::InoxRenderer;
#[cfg(target_arch = "wasm32")]
use inox2d_opengl::OpenglRenderer;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;

#[cfg(target_arch = "wasm32")]
use crate::param_bridge::{extract_available_params, PuppetParam};
#[cfg(target_arch = "wasm32")]
use crate::renderer::{create_renderer, fit_camera_to_puppet, resize_renderer};

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn wasm_init() {
    console_error_panic_hook::set_once();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WaifudexPuppet {
    renderer: OpenglRenderer,
    puppet: Puppet,
    available_params: Vec<PuppetParam>,
    overrides: HashMap<String, Vec2>,
    disposed: bool,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WaifudexPuppet {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement, inp_bytes: &[u8]) -> Result<WaifudexPuppet, JsValue> {
        wasm_init();

        let available_params =
            extract_available_params(inp_bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;
        let model = parse_inp(inp_bytes).map_err(|error| JsValue::from_str(&error.to_string()))?;

        let mut renderer = create_renderer(&canvas)?;
        renderer
            .prepare(&model)
            .map_err(|error| JsValue::from_str(&error.to_string()))?;

        let width = canvas.width().max(1);
        let height = canvas.height().max(1);
        resize_renderer(&mut renderer, width, height);
        fit_camera_to_puppet(&mut renderer, &model.puppet, width, height);

        let mut instance = Self {
            renderer,
            puppet: model.puppet,
            available_params,
            overrides: HashMap::new(),
            disposed: false,
        };
        instance.render_frame(0.0)?;
        Ok(instance)
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), JsValue> {
        self.ensure_live()?;
        resize_renderer(&mut self.renderer, width, height);
        fit_camera_to_puppet(&mut self.renderer, &self.puppet, width, height);
        Ok(())
    }

    pub fn set_param(&mut self, name: &str, x: f32, y: f32) -> bool {
        if self.disposed || self.puppet.get_named_param(name).is_none() {
            return false;
        }

        self.overrides.insert(name.to_string(), Vec2::new(x, y));
        true
    }

    pub fn render_frame(&mut self, dt: f32) -> Result<(), JsValue> {
        self.ensure_live()?;
        self.puppet.begin_set_params();
        for param in &self.available_params {
            let value = self
                .overrides
                .get(&param.name)
                .copied()
                .unwrap_or_else(|| Vec2::new(param.defaults[0], param.defaults[1]));

            if self.puppet.get_named_param(&param.name).is_some() {
                self.puppet.set_named_param(&param.name, value);
            }
        }
        self.puppet.end_set_params(dt.max(0.0));
        self.renderer.clear();
        self.renderer.render(&self.puppet);
        Ok(())
    }

    pub fn get_available_params(&self) -> Result<JsValue, JsValue> {
        self.ensure_live()?;
        serde_wasm_bindgen::to_value(&self.available_params)
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    pub fn dispose(&mut self) {
        self.overrides.clear();
        self.disposed = true;
    }
}

#[cfg(target_arch = "wasm32")]
impl WaifudexPuppet {
    fn ensure_live(&self) -> Result<(), JsValue> {
        if self.disposed {
            return Err(JsValue::from_str("WaifudexPuppet was already disposed"));
        }

        Ok(())
    }
}
