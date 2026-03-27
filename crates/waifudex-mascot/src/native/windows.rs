use std::{
    ffi::{c_void, CStr, CString},
    mem::{size_of, zeroed},
    path::Path,
    ptr::{self, NonNull},
    sync::OnceLock,
};

use inochi2d_sys::{
    inCleanup, inDumpViewport, inParameterSetValue, inPuppetDestroy, inPuppetDraw, inPuppetUpdate,
    inSceneBegin, inSceneDraw, inSceneEnd, inUpdate, inViewportSet, InPuppet,
};
use windows_sys::Win32::{
    Foundation::{FreeLibrary, GetLastError, FARPROC, HMODULE, HWND, LPARAM, LRESULT, WPARAM},
    Graphics::{
        Gdi::{GetDC, ReleaseDC, HDC},
        OpenGL::{
            wglCreateContext, wglDeleteContext, wglGetProcAddress, wglMakeCurrent,
            ChoosePixelFormat, SetPixelFormat, HGLRC, PFD_DOUBLEBUFFER, PFD_DRAW_TO_WINDOW,
            PFD_MAIN_PLANE, PFD_SUPPORT_OPENGL, PFD_TYPE_RGBA, PIXELFORMATDESCRIPTOR,
        },
    },
    System::LibraryLoader::{GetModuleHandleA, GetProcAddress, LoadLibraryA},
    UI::WindowsAndMessaging::{
        CreateWindowExA, DefWindowProcA, DestroyWindow, RegisterClassA, CS_OWNDC, WNDCLASSA,
        WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_OVERLAPPEDWINDOW,
    },
};

use super::{
    apply_camera_for_viewport, initialize_inochi2d, load_params, load_puppet, trace_stage,
    NativeParam,
};
use crate::{MascotError, MascotParamValue, ParamInfo, Result};

const WINDOW_CLASS_NAME: &[u8] = b"WaifudexMascotHiddenWindow\0";
const WGL_CONTEXT_MAJOR_VERSION_ARB: i32 = 0x2091;
const WGL_CONTEXT_MINOR_VERSION_ARB: i32 = 0x2092;
const WGL_CONTEXT_PROFILE_MASK_ARB: i32 = 0x9126;
const WGL_CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB: i32 = 0x0000_0002;

type WglCreateContextAttribsArb = unsafe extern "system" fn(HDC, HGLRC, *const i32) -> HGLRC;

pub struct NativeMascotRenderer {
    window: HiddenWindow,
    context: HGLRC,
    opengl32: HMODULE,
    framebuffer: GlFramebuffer,
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

        trace_stage("register window class");
        register_window_class()?;
        trace_stage("create hidden window");
        let window = HiddenWindow::new()?;

        trace_stage("configure pixel format");
        configure_pixel_format(window.device_context)?;

        trace_stage("create temporary wgl context");
        let temporary_context = unsafe { wglCreateContext(window.device_context) };
        if temporary_context.is_null() {
            return Err(last_win32_error("wglCreateContext failed"));
        }
        make_current(window.device_context, temporary_context)?;

        trace_stage("load opengl32");
        let opengl32 = unsafe { LoadLibraryA(b"opengl32.dll\0".as_ptr()) };
        if opengl32.is_null() {
            return Err(last_win32_error("LoadLibraryA(opengl32.dll) failed"));
        }

        trace_stage("create modern wgl context");
        let context = create_modern_context(window.device_context, temporary_context, opengl32)?;
        make_not_current()?;
        unsafe {
            wglDeleteContext(temporary_context);
        }
        make_current(window.device_context, context)?;

        trace_stage("load gl");
        gl::load_with(|name| load_gl_symbol(opengl32, name));
        ensure_gl_version_at_least(3, 1)?;

        unsafe {
            gl::ClearColor(0.0, 0.0, 0.0, 0.0);
            gl::PixelStorei(gl::PACK_ALIGNMENT, 1);
        }

        let width = width.max(1);
        let height = height.max(1);
        trace_stage("create framebuffer");
        let framebuffer = GlFramebuffer::new(width, height)?;

        trace_stage("init inochi2d");
        initialize_inochi2d(width, height);

        trace_stage("load puppet");
        let puppet = load_puppet(model_path)?;
        trace_stage("load params");
        let params = load_params(puppet);
        let param_infos = params.iter().map(|param| param.info.clone()).collect();
        let frame = vec![0; (width * height * 4) as usize];

        Ok(Self {
            window,
            context,
            opengl32,
            framebuffer,
            puppet,
            params,
            param_infos,
            frame,
            width,
            height,
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

        make_current(self.window.device_context, self.context)?;

        unsafe {
            gl::Viewport(0, 0, self.width as i32, self.height as i32);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);

            inUpdate();
            inSceneBegin();
            inPuppetUpdate(self.puppet.as_ptr());
            inPuppetDraw(self.puppet.as_ptr());
            inSceneEnd();
            inSceneDraw(0.0, 0.0, self.width as f32, self.height as f32);
            inDumpViewport(self.frame.as_mut_ptr(), self.frame.len());
        }

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

        make_current(self.window.device_context, self.context)?;
        self.framebuffer.destroy();
        self.framebuffer = GlFramebuffer::new(width, height)?;
        self.width = width;
        self.height = height;
        self.frame.resize((width * height * 4) as usize, 0);
        unsafe {
            inViewportSet(width, height);
        }
        apply_camera_for_viewport(width, height);
        self.dirty = true;
        Ok(())
    }

    pub fn load_model(&mut self, model_path: &Path) -> Result<()> {
        if !model_path.exists() {
            return Err(MascotError::ModelNotFound(model_path.to_path_buf()));
        }

        make_current(self.window.device_context, self.context)?;
        unsafe {
            inPuppetDestroy(self.puppet.as_ptr());
        }

        let puppet = load_puppet(model_path)?;
        let params = load_params(puppet);
        let param_infos = params.iter().map(|param| param.info.clone()).collect();

        self.puppet = puppet;
        self.params = params;
        self.param_infos = param_infos;
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
        let _ = make_current(self.window.device_context, self.context);
        unsafe {
            inPuppetDestroy(self.puppet.as_ptr());
        }
        self.framebuffer.destroy();
        let _ = make_not_current();
        unsafe {
            wglDeleteContext(self.context);
            if !self.opengl32.is_null() {
                FreeLibrary(self.opengl32);
            }
            inCleanup();
        }
    }
}

struct HiddenWindow {
    handle: HWND,
    device_context: HDC,
}

impl HiddenWindow {
    fn new() -> Result<Self> {
        let instance = unsafe { GetModuleHandleA(ptr::null()) };
        if instance.is_null() {
            return Err(last_win32_error("GetModuleHandleA failed"));
        }

        let handle = unsafe {
            CreateWindowExA(
                0,
                WINDOW_CLASS_NAME.as_ptr(),
                WINDOW_CLASS_NAME.as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_CLIPSIBLINGS | WS_CLIPCHILDREN,
                0,
                0,
                1,
                1,
                ptr::null_mut(),
                ptr::null_mut(),
                instance,
                ptr::null(),
            )
        };
        if handle.is_null() {
            return Err(last_win32_error("CreateWindowExA failed"));
        }

        let device_context = unsafe { GetDC(handle) };
        if device_context.is_null() {
            unsafe {
                DestroyWindow(handle);
            }
            return Err(last_win32_error("GetDC failed"));
        }

        Ok(Self {
            handle,
            device_context,
        })
    }
}

impl Drop for HiddenWindow {
    fn drop(&mut self) {
        unsafe {
            ReleaseDC(self.handle, self.device_context);
            DestroyWindow(self.handle);
        }
    }
}

struct GlFramebuffer {
    handle: u32,
    color_texture: u32,
    depth_stencil: u32,
}

impl GlFramebuffer {
    fn new(width: u32, height: u32) -> Result<Self> {
        let mut handle = 0_u32;
        let mut color_texture = 0_u32;
        let mut depth_stencil = 0_u32;

        unsafe {
            gl::GenFramebuffers(1, &mut handle);
            gl::BindFramebuffer(gl::FRAMEBUFFER, handle);

            gl::GenTextures(1, &mut color_texture);
            gl::BindTexture(gl::TEXTURE_2D, color_texture);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA8 as i32,
                width as i32,
                height as i32,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                ptr::null(),
            );
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                color_texture,
                0,
            );

            gl::GenRenderbuffers(1, &mut depth_stencil);
            gl::BindRenderbuffer(gl::RENDERBUFFER, depth_stencil);
            gl::RenderbufferStorage(
                gl::RENDERBUFFER,
                gl::DEPTH24_STENCIL8,
                width as i32,
                height as i32,
            );
            gl::FramebufferRenderbuffer(
                gl::FRAMEBUFFER,
                gl::DEPTH_STENCIL_ATTACHMENT,
                gl::RENDERBUFFER,
                depth_stencil,
            );

            gl::DrawBuffer(gl::COLOR_ATTACHMENT0);
            gl::ReadBuffer(gl::COLOR_ATTACHMENT0);
        }

        let status = unsafe { gl::CheckFramebufferStatus(gl::FRAMEBUFFER) };
        if status != gl::FRAMEBUFFER_COMPLETE {
            unsafe {
                if depth_stencil != 0 {
                    gl::DeleteRenderbuffers(1, &depth_stencil);
                }
                if color_texture != 0 {
                    gl::DeleteTextures(1, &color_texture);
                }
                if handle != 0 {
                    gl::DeleteFramebuffers(1, &handle);
                }
            }
            return Err(MascotError::NativeContext(format!(
                "framebuffer is incomplete: 0x{status:04x}"
            )));
        }

        Ok(Self {
            handle,
            color_texture,
            depth_stencil,
        })
    }

    fn destroy(&mut self) {
        unsafe {
            if self.depth_stencil != 0 {
                gl::DeleteRenderbuffers(1, &self.depth_stencil);
            }
            if self.color_texture != 0 {
                gl::DeleteTextures(1, &self.color_texture);
            }
            if self.handle != 0 {
                gl::DeleteFramebuffers(1, &self.handle);
            }
        }
        self.depth_stencil = 0;
        self.color_texture = 0;
        self.handle = 0;
    }
}

fn register_window_class() -> Result<()> {
    static WINDOW_CLASS_REGISTRATION: OnceLock<std::result::Result<(), String>> = OnceLock::new();

    match WINDOW_CLASS_REGISTRATION.get_or_init(|| {
        let instance = unsafe { GetModuleHandleA(ptr::null()) };
        if instance.is_null() {
            return Err(format!(
                "GetModuleHandleA failed while registering window class (GetLastError={})",
                unsafe { GetLastError() }
            ));
        }

        let class = WNDCLASSA {
            style: CS_OWNDC,
            lpfnWndProc: Some(hidden_window_proc),
            hInstance: instance,
            lpszClassName: WINDOW_CLASS_NAME.as_ptr(),
            ..unsafe { zeroed() }
        };

        let atom = unsafe { RegisterClassA(&class) };
        if atom == 0 {
            return Err(format!(
                "RegisterClassA failed for hidden OpenGL window (GetLastError={})",
                unsafe { GetLastError() }
            ));
        }

        Ok(())
    }) {
        Ok(()) => Ok(()),
        Err(message) => Err(MascotError::NativeContext(message.clone())),
    }
}

unsafe extern "system" fn hidden_window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcA(window, message, wparam, lparam) }
}

fn configure_pixel_format(device_context: HDC) -> Result<()> {
    let descriptor = PIXELFORMATDESCRIPTOR {
        nSize: size_of::<PIXELFORMATDESCRIPTOR>() as u16,
        nVersion: 1,
        dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
        iPixelType: PFD_TYPE_RGBA,
        cColorBits: 32,
        cAlphaBits: 8,
        cDepthBits: 24,
        cStencilBits: 8,
        iLayerType: PFD_MAIN_PLANE as u8,
        ..unsafe { zeroed() }
    };

    let pixel_format = unsafe { ChoosePixelFormat(device_context, &descriptor) };
    if pixel_format == 0 {
        return Err(last_win32_error("ChoosePixelFormat failed"));
    }

    let set_result = unsafe { SetPixelFormat(device_context, pixel_format, &descriptor) };
    if set_result == 0 {
        return Err(last_win32_error("SetPixelFormat failed"));
    }

    Ok(())
}

fn make_current(device_context: HDC, context: HGLRC) -> Result<()> {
    let result = unsafe { wglMakeCurrent(device_context, context) };
    if result == 0 {
        return Err(last_win32_error("wglMakeCurrent failed"));
    }
    Ok(())
}

fn make_not_current() -> Result<()> {
    let result = unsafe { wglMakeCurrent(ptr::null_mut(), ptr::null_mut()) };
    if result == 0 {
        return Err(last_win32_error("wglMakeCurrent(null) failed"));
    }
    Ok(())
}

fn load_gl_symbol(opengl32: HMODULE, name: &str) -> *const c_void {
    let symbol_name = match CString::new(name) {
        Ok(symbol_name) => symbol_name,
        Err(_) => return ptr::null(),
    };

    let wgl_address = unsafe { wglGetProcAddress(symbol_name.as_ptr().cast()) };
    let wgl_address = function_pointer_to_address(wgl_address);
    if !wgl_address.is_null() && !matches!(wgl_address as usize, 1 | 2 | 3 | usize::MAX) {
        return wgl_address;
    }

    let opengl_address = unsafe { GetProcAddress(opengl32, symbol_name.as_ptr().cast()) };
    function_pointer_to_address(opengl_address)
}

fn create_modern_context(
    device_context: HDC,
    temporary_context: HGLRC,
    opengl32: HMODULE,
) -> Result<HGLRC> {
    let create_context_attribs = load_gl_symbol(opengl32, "wglCreateContextAttribsARB");
    if create_context_attribs.is_null() {
        return Err(MascotError::NativeContext(
            "wglCreateContextAttribsARB is unavailable; OpenGL 3.1+ context required".to_string(),
        ));
    }

    let create_context_attribs: WglCreateContextAttribsArb =
        unsafe { std::mem::transmute(create_context_attribs) };
    let attributes = [
        WGL_CONTEXT_MAJOR_VERSION_ARB,
        3,
        WGL_CONTEXT_MINOR_VERSION_ARB,
        1,
        WGL_CONTEXT_PROFILE_MASK_ARB,
        WGL_CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB,
        0,
    ];

    let context =
        unsafe { create_context_attribs(device_context, temporary_context, attributes.as_ptr()) };
    if context.is_null() {
        return Err(last_win32_error("wglCreateContextAttribsARB failed"));
    }

    Ok(context)
}

fn ensure_gl_version_at_least(min_major: u32, min_minor: u32) -> Result<()> {
    let version_ptr = unsafe { gl::GetString(gl::VERSION) };
    if version_ptr.is_null() {
        return Err(MascotError::NativeContext(
            "glGetString(GL_VERSION) returned null".to_string(),
        ));
    }

    let version = unsafe { CStr::from_ptr(version_ptr.cast()) }
        .to_string_lossy()
        .into_owned();
    let version_token = version.split_whitespace().next().unwrap_or_default();
    let mut parts = version_token.split('.');
    let major = parts
        .next()
        .and_then(|part| part.parse::<u32>().ok())
        .unwrap_or_default();
    let minor = parts
        .next()
        .and_then(|part| part.parse::<u32>().ok())
        .unwrap_or_default();

    if major < min_major || (major == min_major && minor < min_minor) {
        return Err(MascotError::NativeContext(format!(
            "OpenGL {min_major}.{min_minor}+ required, got {version}"
        )));
    }

    Ok(())
}

fn function_pointer_to_address(function: FARPROC) -> *const c_void {
    function.map_or(ptr::null(), |function| function as usize as *const c_void)
}

fn last_win32_error(context: &str) -> MascotError {
    MascotError::NativeContext(format!("{context} (GetLastError={})", unsafe {
        GetLastError()
    }))
}
