use glam::Vec2;
use glow::HasContext;
use inox2d::puppet::Puppet;
use inox2d::render::InoxRenderer;
use inox2d_opengl::OpenglRenderer;
use js_sys::{Object, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext};

pub fn create_renderer(canvas: &HtmlCanvasElement) -> Result<OpenglRenderer, JsValue> {
    let options = Object::new();
    Reflect::set(&options, &JsValue::from_str("alpha"), &JsValue::TRUE)?;
    Reflect::set(
        &options,
        &JsValue::from_str("premultipliedAlpha"),
        &JsValue::TRUE,
    )?;
    Reflect::set(&options, &JsValue::from_str("antialias"), &JsValue::TRUE)?;

    let context = canvas
        .get_context_with_context_options("webgl2", &options.into())?
        .ok_or_else(|| JsValue::from_str("WebGL2 is not available for the mascot canvas"))?
        .dyn_into::<WebGl2RenderingContext>()?;

    let glow = glow::Context::from_webgl2_context(context);
    unsafe {
        glow.clear_color(0.0, 0.0, 0.0, 0.0);
    }

    OpenglRenderer::new(glow).map_err(|error| JsValue::from_str(&error.to_string()))
}

pub fn resize_renderer(renderer: &mut OpenglRenderer, width: u32, height: u32) {
    renderer.resize(width.max(1), height.max(1));
}

pub fn fit_camera_to_puppet<T>(
    renderer: &mut OpenglRenderer,
    puppet: &Puppet<T>,
    width: u32,
    height: u32,
) {
    let mut vertices = puppet.render_ctx.vertex_buffers.verts.iter().skip(4);
    let Some(first) = vertices.next().copied() else {
        resize_renderer(renderer, width, height);
        return;
    };

    let mut min = first;
    let mut max = first;
    for vertex in vertices {
        min.x = min.x.min(vertex.x);
        min.y = min.y.min(vertex.y);
        max.x = max.x.max(vertex.x);
        max.y = max.y.max(vertex.y);
    }

    let model_size = max - min;
    if model_size.x <= f32::EPSILON || model_size.y <= f32::EPSILON {
        resize_renderer(renderer, width, height);
        return;
    }

    let viewport = Vec2::new(width.max(1) as f32, height.max(1) as f32);
    let fit_scale = (viewport.x / model_size.x).min(viewport.y / model_size.y) * 0.82;
    if !fit_scale.is_finite() || fit_scale <= 0.0 {
        resize_renderer(renderer, width, height);
        return;
    }

    renderer.camera.scale = Vec2::splat(fit_scale);
    let visible_size = viewport / renderer.camera.scale;
    renderer.camera.position = ((min + max) * 0.5) - (visible_size * 0.5);
    resize_renderer(renderer, width, height);
}
