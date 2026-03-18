use std::{
    os::raw::c_void,
    path::Path,
    ptr::{self, NonNull},
};

use inochi2d_sys::{
    inCleanup, inParameterSetValue, inPuppetDestroy, inPuppetDraw, inPuppetUpdate, inSceneBegin,
    inSceneDraw, inSceneEnd, inUpdate, inViewportSet, InPuppet,
};
use khronos_egl as egl;

use super::{flip_rows, initialize_inochi2d, load_params, load_puppet, trace_stage, NativeParam};
use crate::{MascotError, MascotParamValue, ParamInfo, Result};

const EGL_PLATFORM_SURFACELESS_MESA: egl::Enum = 0x31DD;

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
            .map_err(map_context_error)?;
        trace_stage("bind api");
        egl.bind_api(egl::OPENGL_API).map_err(map_context_error)?;

        trace_stage("acquire display");
        let display = acquire_display(&egl)?;
        trace_stage("initialize display");
        egl.initialize(display).map_err(map_context_error)?;

        trace_stage("choose config");
        let config = choose_config(&egl, display)?;
        trace_stage("create surface");
        let surface = create_surface(&egl, display, config, width.max(1), height.max(1))?;
        trace_stage("create context");
        let context = create_context(&egl, display, config)?;

        trace_stage("make current");
        egl.make_current(display, Some(surface), Some(surface), Some(context))
            .map_err(map_context_error)?;

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
        initialize_inochi2d(width, height);

        trace_stage("load puppet");
        let puppet = load_puppet(model_path)?;
        trace_stage("load params");
        let params = load_params(puppet);
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
            .map_err(map_context_error)?;

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
            .map_err(map_context_error)?;
        self.egl
            .destroy_surface(self.display, self.surface)
            .map_err(map_context_error)?;

        self.surface = create_surface(&self.egl, self.display, self.config, width, height)?;
        self.egl
            .make_current(
                self.display,
                Some(self.surface),
                Some(self.surface),
                Some(self.context),
            )
            .map_err(map_context_error)?;

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
        .map_err(map_context_error)?
        .ok_or_else(|| MascotError::NativeContext("no compatible egl config".to_string()))
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
        .ok_or_else(|| MascotError::NativeContext("eglGetDisplay returned null".to_string()))
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
        .map_err(map_context_error)
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
        .map_err(map_context_error)
}

fn map_context_error(error: impl ToString) -> MascotError {
    MascotError::NativeContext(error.to_string())
}
