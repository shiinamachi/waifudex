use std::{
    ffi::{CStr, CString},
    path::Path,
    ptr::{self, NonNull},
};

use inochi2d_sys::{
    inCameraGetCurrent, inCameraSetPosition, inCameraSetZoom, inErrorGet, inInit,
    inParameterGetMax, inParameterGetMin, inParameterGetName, inParameterGetValue,
    inParameterIsVec2, inPuppetGetParameters, inPuppetLoad, inViewportSet, InError, InParameter,
    InPuppet,
};

use crate::{MascotError, ParamInfo, Result};

const DEFAULT_CAMERA_REFERENCE_WIDTH: f32 = 630.0;
const DEFAULT_CAMERA_REFERENCE_HEIGHT: f32 = 1080.0;
const DEFAULT_CAMERA_ZOOM: f32 = 0.24;
const DEFAULT_CAMERA_POS_X: f32 = 0.0;
const DEFAULT_CAMERA_POS_Y: f32 = 850.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct CameraFit {
    zoom: f32,
    position_x: f32,
    position_y: f32,
}

#[derive(Clone, Debug)]
pub(super) struct NativeParam {
    pub(super) ptr: NonNull<InParameter>,
    pub(super) info: ParamInfo,
    pub(super) current: (f32, f32),
}

pub(super) fn initialize_inochi2d(width: u32, height: u32) {
    unsafe {
        inInit(None);
        inViewportSet(width.max(1), height.max(1));
    }

    apply_camera_for_viewport(width, height);
}

pub(super) fn load_puppet(model_path: &Path) -> Result<NonNull<InPuppet>> {
    let model_path_cstr = CString::new(model_path.to_string_lossy().as_bytes())
        .map_err(|error| MascotError::NativeFfi(error.to_string()))?;
    NonNull::new(unsafe { inPuppetLoad(model_path_cstr.as_ptr()) }).ok_or_else(last_ffi_error)
}

pub(super) fn load_params(puppet: NonNull<InPuppet>) -> Vec<NativeParam> {
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

    params
}

pub(super) fn last_ffi_error() -> MascotError {
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

pub(super) fn apply_camera_for_viewport(width: u32, height: u32) {
    let fit = camera_fit_for_viewport(width, height);
    let Some(camera) = NonNull::new(unsafe { inCameraGetCurrent() }) else {
        return;
    };

    unsafe {
        inCameraSetZoom(camera.as_ptr(), fit.zoom);
        inCameraSetPosition(camera.as_ptr(), fit.position_x, fit.position_y);
    }
}

fn camera_fit_for_viewport(width: u32, height: u32) -> CameraFit {
    let viewport_width = width.max(1) as f32;
    let viewport_height = height.max(1) as f32;
    let scale = (viewport_width / DEFAULT_CAMERA_REFERENCE_WIDTH)
        .min(viewport_height / DEFAULT_CAMERA_REFERENCE_HEIGHT);

    CameraFit {
        zoom: (DEFAULT_CAMERA_ZOOM * scale).max(f32::EPSILON),
        position_x: DEFAULT_CAMERA_POS_X,
        position_y: DEFAULT_CAMERA_POS_Y,
    }
}

#[cfg(target_os = "linux")]
pub(super) fn flip_rows(rgba: &mut [u8], width: usize, height: usize) {
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

pub(super) fn trace_stage(_stage: &str) {}

#[cfg(test)]
mod tests {
    use super::{
        camera_fit_for_viewport, DEFAULT_CAMERA_POS_X, DEFAULT_CAMERA_POS_Y, DEFAULT_CAMERA_ZOOM,
    };

    #[test]
    fn camera_fit_for_reference_viewport_preserves_default_camera() {
        let fit = camera_fit_for_viewport(630, 1080);

        assert!((fit.zoom - DEFAULT_CAMERA_ZOOM).abs() < 0.0001);
        assert!((fit.position_x - DEFAULT_CAMERA_POS_X).abs() < 0.0001);
        assert!((fit.position_y - DEFAULT_CAMERA_POS_Y).abs() < 0.0001);
    }

    #[test]
    fn camera_fit_for_smaller_viewport_scales_zoom_down_linearly() {
        let fit = camera_fit_for_viewport(420, 720);

        assert!((fit.zoom - 0.16).abs() < 0.0001);
        assert!((fit.position_x - DEFAULT_CAMERA_POS_X).abs() < 0.0001);
        assert!((fit.position_y - DEFAULT_CAMERA_POS_Y).abs() < 0.0001);
    }
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::NativeMascotRenderer;
#[cfg(target_os = "windows")]
pub use windows::NativeMascotRenderer;
