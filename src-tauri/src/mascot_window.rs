#[cfg(windows)]
use std::io;
use std::sync::Mutex;

use tauri::{AppHandle, Manager, Runtime};

const MIN_WINDOW_SIZE: u32 = 180;
const MAX_WINDOW_SIZE: u32 = 1200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MascotWindowSize {
    pub width: u32,
    pub height: u32,
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy)]
struct NativeMascotWindow {
    hwnd: isize,
}

#[derive(Debug)]
pub struct MascotWindowState {
    visible: Mutex<bool>,
    always_on_top: Mutex<bool>,
    #[cfg(windows)]
    window: Mutex<Option<NativeMascotWindow>>,
    size: Mutex<MascotWindowSize>,
}

impl MascotWindowState {
    pub fn new() -> Self {
        Self {
            visible: Mutex::new(false),
            always_on_top: Mutex::new(true),
            #[cfg(windows)]
            window: Mutex::new(None),
            size: Mutex::new(MascotWindowSize {
                width: 420,
                height: 720,
            }),
        }
    }

    pub fn size(&self) -> MascotWindowSize {
        *self
            .size
            .lock()
            .expect("mascot window state mutex poisoned")
    }

    #[cfg(windows)]
    fn attach(&self, window: NativeMascotWindow) {
        *self
            .window
            .lock()
            .expect("mascot window state mutex poisoned") = Some(window);
    }

    pub fn is_initialized(&self) -> bool {
        #[cfg(windows)]
        {
            return self
                .window
                .lock()
                .expect("mascot window state mutex poisoned")
                .is_some();
        }

        #[cfg(not(windows))]
        {
            false
        }
    }

    pub fn is_visible(&self) -> bool {
        *self
            .visible
            .lock()
            .expect("mascot window state mutex poisoned")
    }

    pub fn is_always_on_top(&self) -> bool {
        *self
            .always_on_top
            .lock()
            .expect("mascot window state mutex poisoned")
    }

    pub fn set_always_on_top(&self, always_on_top: bool) -> tauri::Result<()> {
        *self
            .always_on_top
            .lock()
            .expect("mascot window state mutex poisoned") = always_on_top;

        #[cfg(windows)]
        if let Some(window) = *self
            .window
            .lock()
            .expect("mascot window state mutex poisoned")
        {
            apply_window_topmost(
                window.hwnd as windows_sys::Win32::Foundation::HWND,
                always_on_top,
            )
            .map_err(tauri::Error::from)?;
        }

        Ok(())
    }

    pub fn resize(&self, width: u32, height: u32) -> tauri::Result<()> {
        let size = MascotWindowSize {
            width: width.clamp(MIN_WINDOW_SIZE, MAX_WINDOW_SIZE),
            height: height.clamp(MIN_WINDOW_SIZE, MAX_WINDOW_SIZE),
        };

        *self
            .size
            .lock()
            .expect("mascot window state mutex poisoned") = size;

        #[cfg(windows)]
        if let Some(window) = *self
            .window
            .lock()
            .expect("mascot window state mutex poisoned")
        {
            unsafe {
                let _ = windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos(
                    window.hwnd as windows_sys::Win32::Foundation::HWND,
                    std::ptr::null_mut(),
                    0,
                    0,
                    size.width as i32,
                    size.height as i32,
                    windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOMOVE
                        | windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOZORDER
                        | windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE,
                );
            }
        }

        Ok(())
    }

    pub fn resize_limits(&self) -> (u32, u32) {
        (MIN_WINDOW_SIZE, MAX_WINDOW_SIZE)
    }

    pub fn show(&self) {
        #[cfg(windows)]
        if let Some(window) = *self
            .window
            .lock()
            .expect("mascot window state mutex poisoned")
        {
            unsafe {
                let _ = windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(
                    window.hwnd as windows_sys::Win32::Foundation::HWND,
                    windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOW,
                );
            }
        }

        *self
            .visible
            .lock()
            .expect("mascot window state mutex poisoned") = true;
    }

    pub fn hide(&self) {
        #[cfg(windows)]
        if let Some(window) = *self
            .window
            .lock()
            .expect("mascot window state mutex poisoned")
        {
            unsafe {
                let _ = windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(
                    window.hwnd as windows_sys::Win32::Foundation::HWND,
                    windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE,
                );
            }
        }

        *self
            .visible
            .lock()
            .expect("mascot window state mutex poisoned") = false;
    }

    pub fn drag(&self) {}
}

pub fn initialize<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let state = app.state::<MascotWindowState>();
    if state.is_initialized() {
        return Ok(());
    }

    #[cfg(windows)]
    {
        let size = state.size();
        let hwnd = create_layered_mascot_window(
            size.width as i32,
            size.height as i32,
            state.is_always_on_top(),
        )
        .map_err(tauri::Error::from)?;
        state.attach(NativeMascotWindow {
            hwnd: hwnd as isize,
        });
    }

    #[cfg(not(windows))]
    {
        let _ = app;
    }

    Ok(())
}

pub fn set_always_on_top<R: Runtime>(app: &AppHandle<R>, always_on_top: bool) -> tauri::Result<()> {
    if let Some(state) = app.try_state::<MascotWindowState>() {
        state.set_always_on_top(always_on_top)?;
    }

    Ok(())
}

pub fn resize<R: Runtime>(app: &AppHandle<R>, width: u32, height: u32) -> tauri::Result<()> {
    if let Some(state) = app.try_state::<MascotWindowState>() {
        state.resize(width, height)?;
    }

    Ok(())
}

pub fn present_frame<R: Runtime>(
    app: &AppHandle<R>,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> tauri::Result<()> {
    #[cfg(windows)]
    {
        let state = app.state::<MascotWindowState>();
        let window = *state
            .window
            .lock()
            .expect("mascot window state mutex poisoned");
        if let Some(window) = window {
            present_frame_windows(
                window.hwnd as windows_sys::Win32::Foundation::HWND,
                width,
                height,
                rgba,
            )?;
        }
    }

    #[cfg(not(windows))]
    {
        let _ = (app, width, height, rgba);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mascot_window_state_defaults_to_always_on_top() {
        let state = MascotWindowState::new();
        assert!(state.is_always_on_top());
    }

    #[test]
    fn mascot_window_state_updates_always_on_top_flag() {
        let state = MascotWindowState::new();
        state.set_always_on_top(false).unwrap();
        assert!(!state.is_always_on_top());
    }
}

#[cfg(windows)]
fn create_layered_mascot_window(
    width: i32,
    height: i32,
    always_on_top: bool,
) -> io::Result<windows_sys::Win32::Foundation::HWND> {
    use windows_sys::Win32::{
        Foundation::HWND,
        System::LibraryLoader::GetModuleHandleA,
        UI::WindowsAndMessaging::{
            CreateWindowExA, LoadCursorW, RegisterClassA, CS_HREDRAW, CS_VREDRAW, IDC_ARROW,
            WNDCLASSA, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
        },
    };

    static WINDOW_CLASS: &[u8] = b"WaifudexMascotLayeredWindow\0";

    unsafe {
        let instance = GetModuleHandleA(std::ptr::null());
        let cursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);

        let class = WNDCLASSA {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(mascot_window_proc),
            hInstance: instance,
            lpszClassName: WINDOW_CLASS.as_ptr(),
            hCursor: cursor,
            ..std::mem::zeroed()
        };

        let _ = RegisterClassA(&class);

        let mut extended_style = WS_EX_LAYERED | WS_EX_TOOLWINDOW;
        if always_on_top {
            extended_style |= WS_EX_TOPMOST;
        }

        let hwnd: HWND = CreateWindowExA(
            extended_style,
            WINDOW_CLASS.as_ptr(),
            WINDOW_CLASS.as_ptr(),
            WS_POPUP,
            160,
            160,
            width,
            height,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        );

        if hwnd.is_null() {
            return Err(io::Error::other("CreateWindowExA failed"));
        }

        Ok(hwnd)
    }
}

#[cfg(windows)]
fn apply_window_topmost(
    hwnd: windows_sys::Win32::Foundation::HWND,
    always_on_top: bool,
) -> io::Result<()> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    };

    let insert_after = if always_on_top {
        HWND_TOPMOST
    } else {
        HWND_NOTOPMOST
    };
    let result = unsafe {
        SetWindowPos(
            hwnd,
            insert_after,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    };

    if result == 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(())
}

#[cfg(windows)]
unsafe extern "system" fn mascot_window_proc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DefWindowProcA, SendMessageA, HTCAPTION, WM_LBUTTONDOWN, WM_NCLBUTTONDOWN,
    };

    if message == WM_LBUTTONDOWN {
        let _ = SendMessageA(hwnd, WM_NCLBUTTONDOWN, HTCAPTION as usize, 0);
        return 0;
    }

    DefWindowProcA(hwnd, message, wparam, lparam)
}

#[cfg(windows)]
fn present_frame_windows(
    hwnd: windows_sys::Win32::Foundation::HWND,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> tauri::Result<()> {
    use windows_sys::Win32::{
        Foundation::{GetLastError, POINT, SIZE},
        Graphics::Gdi::{
            CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject,
            AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION,
            DIB_RGB_COLORS, HGDIOBJ,
        },
        UI::WindowsAndMessaging::{UpdateLayeredWindow, ULW_ALPHA},
    };

    let memory_dc = unsafe { CreateCompatibleDC(std::ptr::null_mut()) };
    if memory_dc.is_null() {
        return Ok(());
    }

    let bitmap_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut bits = std::ptr::null_mut();
    let bitmap = unsafe {
        CreateDIBSection(
            memory_dc,
            &bitmap_info,
            DIB_RGB_COLORS,
            &mut bits,
            std::ptr::null_mut(),
            0,
        )
    };
    if bitmap.is_null() || bits.is_null() {
        unsafe {
            DeleteDC(memory_dc);
        }
        return Ok(());
    }

    let previous = unsafe { SelectObject(memory_dc, bitmap as HGDIOBJ) };

    let pixel_count = (width as usize) * (height as usize);
    let buffer = unsafe { std::slice::from_raw_parts_mut(bits.cast::<u8>(), pixel_count * 4) };
    for index in 0..pixel_count {
        let src = index * 4;
        let dst = index * 4;
        let r = rgba[src] as u32;
        let g = rgba[src + 1] as u32;
        let b = rgba[src + 2] as u32;
        let a = rgba[src + 3] as u32;

        buffer[dst] = ((b * a) / 255) as u8;
        buffer[dst + 1] = ((g * a) / 255) as u8;
        buffer[dst + 2] = ((r * a) / 255) as u8;
        buffer[dst + 3] = a as u8;
    }

    let size = SIZE {
        cx: width as i32,
        cy: height as i32,
    };
    let src_point = POINT { x: 0, y: 0 };
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };

    unsafe {
        let result = UpdateLayeredWindow(
            hwnd,
            std::ptr::null_mut(),
            std::ptr::null(),
            &size,
            memory_dc,
            &src_point,
            0,
            &blend,
            ULW_ALPHA,
        );
        if result == 0 {
            eprintln!(
                "waifudex mascot: UpdateLayeredWindow failed hwnd={} size={}x{} err={}",
                hwnd as isize,
                width,
                height,
                GetLastError()
            );
        }
        SelectObject(memory_dc, previous);
        DeleteObject(bitmap as HGDIOBJ);
        DeleteDC(memory_dc);
    }

    Ok(())
}
