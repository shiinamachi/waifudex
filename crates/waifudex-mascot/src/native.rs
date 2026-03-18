use std::{
    ffi::{CStr, CString},
    os::raw::c_void,
    path::Path,
    ptr::{self, NonNull},
};

use inochi2d_sys::{
    inCameraGetCurrent, inCameraSetPosition, inCameraSetZoom, inCleanup, inErrorGet, inInit,
    inParameterGetMax, inParameterGetMin, inParameterGetName, inParameterGetValue,
    inParameterIsVec2, inParameterSetValue, inPuppetDestroy, inPuppetDraw, inPuppetGetParameters,
    inPuppetLoad, inPuppetUpdate, inSceneBegin, inSceneDraw, inSceneEnd, inUpdate, inViewportSet,
    InError, InParameter, InPuppet,
};
use khronos_egl as egl;

use crate::{MascotError, MascotParamValue, ParamInfo, Result};

const EGL_PLATFORM_SURFACELESS_MESA: egl::Enum = 0x31DD;
const CAMERA_ZOOM: f32 = 0.24;
const CAMERA_POS_X: f32 = 0.0;
const CAMERA_POS_Y: f32 = 850.0;

#[derive(Clone, Debug)]
struct NativeParam {
    ptr: NonNull<InParameter>,
    info: ParamInfo,
    current: (f32, f32),
}

pub struct NativeMascotRenderer {
    egl: egl::DynamicInstance,
    display: egl::Display,
    config: egl::Config,
    context: egl::Context,
    surface: egl::Surface,
    puppet: NonNull<InPuppet>,
    params: Vec<NativeParam>,
    param_infos: Vec<ParamInfo>,
    frame: Vec<u8>,
    width: u32,
    height: u32,
    revision: u64,
    dirty: bool,
}

unsafe impl Send for NativeMascotRenderer {}

impl NativeMascotRenderer {
    pub fn new(model_path: &Path, width: u32, height: u32) -> Result<Self> {
        if !model_path.exists() {
            return Err(MascotError::ModelNotFound(model_path.to_path_buf()));
        }

        trace_stage("load egl");
        let egl = unsafe { egl::DynamicInstance::<egl::Latest>::load_required() }
            .map_err(map_egl_error)?;
        trace_stage("bind api");
        egl.bind_api(egl::OPENGL_API).map_err(map_egl_error)?;

        trace_stage("acquire display");
        let display = acquire_display(&egl)?;
        trace_stage("initialize display");
        egl.initialize(display).map_err(map_egl_error)?;

        trace_stage("choose config");
        let config = choose_config(&egl, display)?;
        trace_stage("create surface");
        let surface = create_surface(&egl, display, config, width.max(1), height.max(1))?;
        trace_stage("create context");
        let context = create_context(&egl, display, config)?;

        trace_stage("make current");
        egl.make_current(display, Some(surface), Some(surface), Some(context))
            .map_err(map_egl_error)?;

        trace_stage("load gl");
        gl::load_with(|name| {
            egl.get_proc_address(name)
                .map(|proc| proc as *const c_void)
                .unwrap_or(ptr::null())
        });

        unsafe {
            gl::Viewport(0, 0, width.max(1) as i32, height.max(1) as i32);
            gl::ClearColor(0.0, 0.0, 0.0, 0.0);
            gl::PixelStorei(gl::PACK_ALIGNMENT, 1);
        }

        trace_stage("init inochi2d");
        unsafe {
            inInit(None);
            inViewportSet(width.max(1), height.max(1));
        }

        trace_stage("camera");
        let camera = unsafe { inCameraGetCurrent() };
        if !camera.is_null() {
            unsafe {
                inCameraSetZoom(camera, CAMERA_ZOOM);
                inCameraSetPosition(camera, CAMERA_POS_X, CAMERA_POS_Y);
            }
        }

        trace_stage("load puppet");
        let model_path_cstr = CString::new(model_path.to_string_lossy().as_bytes())
            .map_err(|error| MascotError::NativeFfi(error.to_string()))?;
        let puppet = NonNull::new(unsafe { inPuppetLoad(model_path_cstr.as_ptr()) })
            .ok_or_else(last_ffi_error)?;
        trace_stage("load params");
        let params = load_params(puppet)?;
        let param_infos = params.iter().map(|param| param.info.clone()).collect();
        let frame = vec![0; (width.max(1) * height.max(1) * 4) as usize];

        Ok(Self {
            egl,
            display,
            config,
            context,
            surface,
            puppet,
            params,
            param_infos,
            frame,
            width: width.max(1),
            height: height.max(1),
            revision: 0,
            dirty: true,
        })
    }

    pub fn set_param(&mut self, param: &MascotParamValue) -> bool {
        let Some(existing) = self
            .params
            .iter_mut()
            .find(|entry| entry.info.name == param.name)
        else {
            return false;
        };

        if existing.current == (param.x, param.y) {
            return false;
        }

        unsafe {
            inParameterSetValue(existing.ptr.as_ptr(), param.x, param.y);
        }
        existing.current = (param.x, param.y);
        self.dirty = true;
        true
    }

    pub fn render_frame(&mut self, _dt: f32) -> Result<Option<&[u8]>> {
        if !self.dirty {
            return Ok(None);
        }

        self.egl
            .make_current(
                self.display,
                Some(self.surface),
                Some(self.surface),
                Some(self.context),
            )
            .map_err(map_egl_error)?;

        unsafe {
            gl::Viewport(0, 0, self.width as i32, self.height as i32);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);

            inUpdate();
            inSceneBegin();
            inPuppetUpdate(self.puppet.as_ptr());
            inPuppetDraw(self.puppet.as_ptr());
            inSceneEnd();
            inSceneDraw(0.0, 0.0, self.width as f32, self.height as f32);

            gl::ReadPixels(
                0,
                0,
                self.width as i32,
                self.height as i32,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                self.frame.as_mut_ptr() as *mut c_void,
            );
        }

        flip_rows(&mut self.frame, self.width as usize, self.height as usize);
        self.revision = self.revision.saturating_add(1);
        self.dirty = false;
        Ok(Some(&self.frame))
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        let width = width.max(1);
        let height = height.max(1);
        if self.width == width && self.height == height {
            return Ok(());
        }

        self.egl
            .make_current(self.display, None, None, None)
            .map_err(map_egl_error)?;
        self.egl
            .destroy_surface(self.display, self.surface)
            .map_err(map_egl_error)?;

        self.surface = create_surface(&self.egl, self.display, self.config, width, height)?;
        self.egl
            .make_current(
                self.display,
                Some(self.surface),
                Some(self.surface),
                Some(self.context),
            )
            .map_err(map_egl_error)?;

        self.width = width;
        self.height = height;
        self.frame.resize((width * height * 4) as usize, 0);
        unsafe {
            gl::Viewport(0, 0, width as i32, height as i32);
            inViewportSet(width, height);
        }
        self.dirty = true;
        Ok(())
    }

    pub fn available_params(&self) -> &[ParamInfo] {
        &self.param_infos
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }
}

impl Drop for NativeMascotRenderer {
    fn drop(&mut self) {
        unsafe {
            inPuppetDestroy(self.puppet.as_ptr());
        }
        let _ = self.egl.make_current(self.display, None, None, None);
        let _ = self.egl.destroy_context(self.display, self.context);
        let _ = self.egl.destroy_surface(self.display, self.surface);
        let _ = self.egl.terminate(self.display);
        unsafe {
            inCleanup();
        }
    }
}

fn choose_config(egl: &egl::DynamicInstance, display: egl::Display) -> Result<egl::Config> {
    let attributes = [
        egl::SURFACE_TYPE,
        egl::PBUFFER_BIT,
        egl::RENDERABLE_TYPE,
        egl::OPENGL_BIT,
        egl::RED_SIZE,
        8,
        egl::GREEN_SIZE,
        8,
        egl::BLUE_SIZE,
        8,
        egl::ALPHA_SIZE,
        8,
        egl::STENCIL_SIZE,
        8,
        egl::NONE,
    ];

    egl.choose_first_config(display, &attributes)
        .map_err(map_egl_error)?
        .ok_or_else(|| MascotError::NativeEgl("no compatible egl config".to_string()))
}

fn acquire_display(egl: &egl::DynamicInstance) -> Result<egl::Display> {
    let surfaceless_attrs = [egl::ATTRIB_NONE];
    let surfaceless = unsafe {
        egl.get_platform_display(
            EGL_PLATFORM_SURFACELESS_MESA,
            ptr::null_mut(),
            &surfaceless_attrs,
        )
    };
    if let Ok(display) = surfaceless {
        return Ok(display);
    }

    unsafe { egl.get_display(egl::DEFAULT_DISPLAY) }
        .ok_or_else(|| MascotError::NativeEgl("eglGetDisplay returned null".to_string()))
}

fn create_context(
    egl: &egl::DynamicInstance,
    display: egl::Display,
    config: egl::Config,
) -> Result<egl::Context> {
    let attributes = [
        egl::CONTEXT_MAJOR_VERSION,
        3,
        egl::CONTEXT_MINOR_VERSION,
        1,
        egl::NONE,
    ];

    egl.create_context(display, config, None, &attributes)
        .map_err(map_egl_error)
}

fn create_surface(
    egl: &egl::DynamicInstance,
    display: egl::Display,
    config: egl::Config,
    width: u32,
    height: u32,
) -> Result<egl::Surface> {
    let attributes = [
        egl::WIDTH,
        width as i32,
        egl::HEIGHT,
        height as i32,
        egl::NONE,
    ];

    egl.create_pbuffer_surface(display, config, &attributes)
        .map_err(map_egl_error)
}

fn load_params(puppet: NonNull<InPuppet>) -> Result<Vec<NativeParam>> {
    let mut length = 0_usize;
    unsafe {
        inPuppetGetParameters(puppet.as_ptr(), ptr::null_mut(), &mut length);
    }

    let mut raw_params = vec![ptr::null_mut::<InParameter>(); length];
    let mut raw_params_ptr = raw_params.as_mut_ptr();
    unsafe {
        inPuppetGetParameters(puppet.as_ptr(), &mut raw_params_ptr, &mut length);
    }
    raw_params.truncate(length);

    let mut params = Vec::with_capacity(raw_params.len());
    for raw in raw_params {
        let Some(ptr) = NonNull::new(raw) else {
            continue;
        };

        let name_ptr = unsafe { inParameterGetName(ptr.as_ptr()) };
        let name = if name_ptr.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(name_ptr) }
                .to_string_lossy()
                .into_owned()
        };

        let mut current_x = 0.0_f32;
        let mut current_y = 0.0_f32;
        let mut min_x = 0.0_f32;
        let mut min_y = 0.0_f32;
        let mut max_x = 0.0_f32;
        let mut max_y = 0.0_f32;
        unsafe {
            inParameterGetValue(ptr.as_ptr(), &mut current_x, &mut current_y);
            inParameterGetMin(ptr.as_ptr(), &mut min_x, &mut min_y);
            inParameterGetMax(ptr.as_ptr(), &mut max_x, &mut max_y);
        }

        params.push(NativeParam {
            ptr,
            info: ParamInfo {
                name,
                is_vec2: unsafe { inParameterIsVec2(ptr.as_ptr()) },
                min: [min_x, min_y],
                max: [max_x, max_y],
                defaults: [current_x, current_y],
            },
            current: (current_x, current_y),
        });
    }

    Ok(params)
}

fn last_ffi_error() -> MascotError {
    let error = unsafe { inErrorGet() };
    if error.is_null() {
        return MascotError::NativeFfi("inochi2d returned a null pointer".to_string());
    }

    let error = unsafe { &*error.cast::<InError>() };
    if error.msg.is_null() || error.len == 0 {
        return MascotError::NativeFfi("inochi2d call failed".to_string());
    }

    let message = unsafe { std::slice::from_raw_parts(error.msg.cast::<u8>(), error.len) };
    MascotError::NativeFfi(String::from_utf8_lossy(message).into_owned())
}

fn map_egl_error(error: impl ToString) -> MascotError {
    MascotError::NativeEgl(error.to_string())
}

fn trace_stage(_stage: &str) {
    #[cfg(test)]
    eprintln!("[waifudex-mascot] {_stage}");
}

fn flip_rows(rgba: &mut [u8], width: usize, height: usize) {
    let stride = width * 4;
    let mut row = vec![0_u8; stride];
    for y in 0..(height / 2) {
        let top = y * stride;
        let bottom = (height - y - 1) * stride;
        row.copy_from_slice(&rgba[top..top + stride]);
        rgba.copy_within(bottom..bottom + stride, top);
        rgba[bottom..bottom + stride].copy_from_slice(&row);
    }
}
