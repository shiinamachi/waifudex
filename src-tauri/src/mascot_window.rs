#[cfg(windows)]
#[cfg(windows)]
use std::collections::BTreeMap;
#[cfg(windows)]
use std::io;
use std::sync::Mutex;

use crate::app_settings::CharacterWindowPosition;
use crate::contracts::DisplayMonitorOption;
use tauri::{AppHandle, Manager, Runtime};

const MIN_WINDOW_SIZE: u32 = 180;
const MAX_WINDOW_SIZE: u32 = 1200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MascotWindowSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MascotWindowPlacement {
    pub monitor_id: Option<String>,
    pub position: Option<CharacterWindowPosition>,
}

#[cfg_attr(not(any(test, windows)), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MonitorWorkArea {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[cfg_attr(not(any(test, windows)), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct DisplayMonitorSnapshot {
    option: DisplayMonitorOption,
    work_area: MonitorWorkArea,
    is_primary: bool,
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy)]
struct NativeMascotWindow {
    hwnd: isize,
}

#[cfg(windows)]
struct NativeMascotWindowContext {
    on_window_pos_changed:
        Box<dyn Fn(Option<String>, Option<CharacterWindowPosition>) + Send + Sync>,
}

#[derive(Debug)]
pub struct MascotWindowState {
    visible: Mutex<bool>,
    always_on_top: Mutex<bool>,
    #[cfg(windows)]
    window: Mutex<Option<NativeMascotWindow>>,
    #[cfg(windows)]
    monitor_move_suppressed: Mutex<bool>,
    size: Mutex<MascotWindowSize>,
}

impl MascotWindowState {
    pub fn new() -> Self {
        Self {
            visible: Mutex::new(false),
            always_on_top: Mutex::new(true),
            #[cfg(windows)]
            window: Mutex::new(None),
            #[cfg(windows)]
            monitor_move_suppressed: Mutex::new(false),
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

    #[cfg(windows)]
    fn is_monitor_move_suppressed(&self) -> bool {
        *self
            .monitor_move_suppressed
            .lock()
            .expect("mascot window state mutex poisoned")
    }

    #[cfg(windows)]
    fn set_monitor_move_suppressed(&self, value: bool) {
        *self
            .monitor_move_suppressed
            .lock()
            .expect("mascot window state mutex poisoned") = value;
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
        attach_window_context(hwnd, app);
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

pub fn available_display_monitors<R: Runtime>(
    app: &AppHandle<R>,
) -> tauri::Result<Vec<DisplayMonitorOption>> {
    #[cfg(windows)]
    {
        let _ = app;
        return list_display_monitors_windows()
            .map(|monitors| monitors.into_iter().map(|monitor| monitor.option).collect())
            .map_err(tauri::Error::from);
    }

    #[cfg(not(windows))]
    {
        let _ = app;
        Ok(Vec::new())
    }
}

pub fn current_display_monitor_id<R: Runtime>(
    _app: &AppHandle<R>,
) -> tauri::Result<Option<String>> {
    #[cfg(windows)]
    {
        if let Some(state) = _app.try_state::<MascotWindowState>() {
            let window = *state
                .window
                .lock()
                .expect("mascot window state mutex poisoned");
            if let Some(window) = window {
                return current_monitor_id_for_hwnd(
                    window.hwnd as windows_sys::Win32::Foundation::HWND,
                )
                .map_err(tauri::Error::from);
            }
        }
    }

    Ok(None)
}

pub fn current_window_position<R: Runtime>(
    _app: &AppHandle<R>,
) -> tauri::Result<Option<CharacterWindowPosition>> {
    #[cfg(windows)]
    {
        if let Some(state) = _app.try_state::<MascotWindowState>() {
            let window = *state
                .window
                .lock()
                .expect("mascot window state mutex poisoned");
            if let Some(window) = window {
                return current_window_position_for_hwnd(
                    window.hwnd as windows_sys::Win32::Foundation::HWND,
                )
                .map(Some)
                .map_err(tauri::Error::from);
            }
        }
    }

    Ok(None)
}

pub fn move_to_monitor<R: Runtime>(
    _app: &AppHandle<R>,
    requested_monitor_id: Option<&str>,
) -> tauri::Result<MascotWindowPlacement> {
    #[cfg(windows)]
    {
        if let Some(state) = _app.try_state::<MascotWindowState>() {
            let window = *state
                .window
                .lock()
                .expect("mascot window state mutex poisoned");
            if let Some(window) = window {
                return move_window_to_monitor_windows(
                    &state,
                    window.hwnd as windows_sys::Win32::Foundation::HWND,
                    requested_monitor_id,
                )
                .map_err(tauri::Error::from);
            }
        }
    }

    Ok(MascotWindowPlacement {
        monitor_id: requested_monitor_id.map(str::to_string),
        position: None,
    })
}

pub fn restore_window_placement<R: Runtime>(
    _app: &AppHandle<R>,
    requested_monitor_id: Option<&str>,
    saved_position: Option<CharacterWindowPosition>,
) -> tauri::Result<MascotWindowPlacement> {
    #[cfg(windows)]
    {
        if let Some(state) = _app.try_state::<MascotWindowState>() {
            let window = *state
                .window
                .lock()
                .expect("mascot window state mutex poisoned");
            if let Some(window) = window {
                return restore_window_placement_windows(
                    &state,
                    window.hwnd as windows_sys::Win32::Foundation::HWND,
                    requested_monitor_id,
                    saved_position,
                )
                .map_err(tauri::Error::from);
            }
        }
    }

    Ok(MascotWindowPlacement {
        monitor_id: requested_monitor_id.map(str::to_string),
        position: saved_position,
    })
}

pub fn move_to_position<R: Runtime>(
    _app: &AppHandle<R>,
    requested_position: CharacterWindowPosition,
    requested_monitor_id: Option<&str>,
) -> tauri::Result<MascotWindowPlacement> {
    #[cfg(windows)]
    {
        return run_move_to_position_on_main_thread(
            _app,
            requested_position,
            requested_monitor_id.map(str::to_string),
        );
    }

    Ok(MascotWindowPlacement {
        monitor_id: requested_monitor_id.map(str::to_string),
        position: Some(requested_position),
    })
}

#[tauri::command]
pub fn get_display_monitors(app: AppHandle) -> Result<Vec<DisplayMonitorOption>, String> {
    available_display_monitors(&app).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn move_character_window_command(app: AppHandle, x: i32, y: i32) -> Result<(), String> {
    eprintln!("waifudex move_character_window_command: x={x} y={y}");
    crate::app_settings::update_app_settings(
        &app,
        crate::app_settings::AppSettingsUpdate {
            character_window_position: Some(CharacterWindowPosition { x, y }),
            ..Default::default()
        },
    )
    .map(|_| ())
    .map_err(|error| error.to_string())
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
    use crate::app_settings::CharacterWindowPosition;

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

    #[test]
    fn resolve_requested_monitor_prefers_requested_id() {
        let monitors = vec![
            test_monitor("\\\\.\\DISPLAY1", true),
            test_monitor("\\\\.\\DISPLAY2", false),
        ];

        assert_eq!(
            resolve_monitor_id(Some("\\\\.\\DISPLAY2"), Some("\\\\.\\DISPLAY1"), &monitors)
                .as_deref(),
            Some("\\\\.\\DISPLAY2")
        );
    }

    #[test]
    fn resolve_requested_monitor_falls_back_to_primary() {
        let monitors = vec![
            test_monitor("\\\\.\\DISPLAY1", true),
            test_monitor("\\\\.\\DISPLAY2", false),
        ];

        assert_eq!(
            resolve_monitor_id(Some("\\\\.\\MISSING"), Some("\\\\.\\DISPLAY2"), &monitors)
                .as_deref(),
            Some("\\\\.\\DISPLAY1")
        );
    }

    #[test]
    fn display_monitor_label_prefers_friendly_name() {
        assert_eq!(
            display_monitor_label("\\\\.\\DISPLAY1", Some("DELL U2720Q"), None, false),
            "DELL U2720Q"
        );
    }

    #[test]
    fn display_monitor_label_falls_back_to_device_name() {
        assert_eq!(
            display_monitor_label("\\\\.\\DISPLAY1", None, None, true),
            "\\\\.\\DISPLAY1 (Primary)"
        );
    }

    #[test]
    fn display_monitor_label_prefers_readable_fallback_for_builtin_display() {
        assert_eq!(
            display_monitor_label("\\\\.\\DISPLAY22", None, Some("Built-in Display"), false),
            "Built-in Display"
        );
    }

    #[test]
    fn clamp_window_position_keeps_window_inside_monitor_work_area() {
        let position = clamp_window_position_to_work_area(
            CharacterWindowPosition { x: 1_850, y: 900 },
            MascotWindowSize {
                width: 420,
                height: 720,
            },
            MonitorWorkArea {
                left: 0,
                top: 0,
                right: 1_920,
                bottom: 1_080,
            },
        );

        assert_eq!(position, CharacterWindowPosition { x: 1_500, y: 360 });
    }

    #[test]
    fn resolve_restored_window_position_keeps_saved_position_on_same_monitor() {
        let monitors = vec![
            test_monitor("\\\\.\\DISPLAY1", true),
            DisplayMonitorSnapshot {
                option: DisplayMonitorOption {
                    id: "\\\\.\\DISPLAY2".to_string(),
                    label: "\\\\.\\DISPLAY2".to_string(),
                    work_area_left: 1_920,
                    work_area_top: 0,
                    work_area_width: 1_920,
                    work_area_height: 1_080,
                },
                work_area: MonitorWorkArea {
                    left: 1_920,
                    top: 0,
                    right: 3_840,
                    bottom: 1_080,
                },
                is_primary: false,
            },
        ];

        let position = resolve_restored_window_position(
            Some("\\\\.\\DISPLAY2"),
            Some(CharacterWindowPosition { x: 2_400, y: 200 }),
            MascotWindowSize {
                width: 420,
                height: 720,
            },
            &monitors,
        )
        .expect("expected a restored position");

        assert_eq!(position, CharacterWindowPosition { x: 2_400, y: 200 });
    }

    #[test]
    fn resolve_restored_window_position_recenters_for_new_monitor() {
        let monitors = vec![
            test_monitor("\\\\.\\DISPLAY1", true),
            DisplayMonitorSnapshot {
                option: DisplayMonitorOption {
                    id: "\\\\.\\DISPLAY2".to_string(),
                    label: "\\\\.\\DISPLAY2".to_string(),
                    work_area_left: 1_920,
                    work_area_top: 0,
                    work_area_width: 1_920,
                    work_area_height: 1_080,
                },
                work_area: MonitorWorkArea {
                    left: 1_920,
                    top: 0,
                    right: 3_840,
                    bottom: 1_080,
                },
                is_primary: false,
            },
        ];

        let position = resolve_restored_window_position(
            Some("\\\\.\\DISPLAY1"),
            Some(CharacterWindowPosition { x: 2_400, y: 200 }),
            MascotWindowSize {
                width: 420,
                height: 720,
            },
            &monitors,
        )
        .expect("expected a restored position");

        assert_eq!(position, CharacterWindowPosition { x: 750, y: 180 });
    }

    #[test]
    fn clamp_requested_window_position_keeps_requested_position_when_it_fits() {
        let position = clamp_requested_window_position(
            CharacterWindowPosition { x: 2400, y: 180 },
            MascotWindowSize {
                width: 420,
                height: 720,
            },
            MonitorWorkArea {
                left: 1920,
                top: 0,
                right: 3840,
                bottom: 1080,
            },
        );

        assert_eq!(position, CharacterWindowPosition { x: 2400, y: 180 });
    }

    #[test]
    fn clamp_requested_window_position_clamps_to_monitor_edges() {
        let position = clamp_requested_window_position(
            CharacterWindowPosition { x: 3800, y: 900 },
            MascotWindowSize {
                width: 420,
                height: 720,
            },
            MonitorWorkArea {
                left: 1920,
                top: 0,
                right: 3840,
                bottom: 1080,
            },
        );

        assert_eq!(position, CharacterWindowPosition { x: 3420, y: 360 });
    }

    fn test_monitor(id: &str, is_primary: bool) -> DisplayMonitorSnapshot {
        DisplayMonitorSnapshot {
            option: DisplayMonitorOption {
                id: id.to_string(),
                label: id.to_string(),
                work_area_left: 0,
                work_area_top: 0,
                work_area_width: 1_920,
                work_area_height: 1_080,
            },
            work_area: MonitorWorkArea {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            },
            is_primary,
        }
    }
}

#[cfg_attr(not(any(test, windows)), allow(dead_code))]
fn resolve_monitor_id(
    requested_monitor_id: Option<&str>,
    current_monitor_id: Option<&str>,
    monitors: &[DisplayMonitorSnapshot],
) -> Option<String> {
    if let Some(requested_monitor_id) = requested_monitor_id {
        if let Some(monitor) = monitors
            .iter()
            .find(|monitor| monitor.option.id == requested_monitor_id)
        {
            return Some(monitor.option.id.clone());
        }
    } else if let Some(current_monitor_id) = current_monitor_id {
        if let Some(monitor) = monitors
            .iter()
            .find(|monitor| monitor.option.id == current_monitor_id)
        {
            return Some(monitor.option.id.clone());
        }
    }

    monitors
        .iter()
        .find(|monitor| monitor.is_primary)
        .or_else(|| monitors.first())
        .map(|monitor| monitor.option.id.clone())
}

#[cfg_attr(not(any(test, windows)), allow(dead_code))]
fn place_window_in_work_area(size: MascotWindowSize, work_area: MonitorWorkArea) -> (i32, i32) {
    let width = size.width as i32;
    let height = size.height as i32;
    let available_width = (work_area.right - work_area.left).max(0);
    let available_height = (work_area.bottom - work_area.top).max(0);

    (
        work_area.left + ((available_width - width).max(0) / 2),
        work_area.top + ((available_height - height).max(0) / 2),
    )
}

#[cfg_attr(not(any(test, windows)), allow(dead_code))]
fn clamp_window_position_to_work_area(
    position: CharacterWindowPosition,
    size: MascotWindowSize,
    work_area: MonitorWorkArea,
) -> CharacterWindowPosition {
    let max_x = work_area.left + ((work_area.right - work_area.left) - size.width as i32).max(0);
    let max_y = work_area.top + ((work_area.bottom - work_area.top) - size.height as i32).max(0);

    CharacterWindowPosition {
        x: position.x.clamp(work_area.left, max_x),
        y: position.y.clamp(work_area.top, max_y),
    }
}

#[cfg_attr(not(any(test, windows)), allow(dead_code))]
fn clamp_requested_window_position(
    position: CharacterWindowPosition,
    size: MascotWindowSize,
    work_area: MonitorWorkArea,
) -> CharacterWindowPosition {
    clamp_window_position_to_work_area(position, size, work_area)
}

#[cfg_attr(not(any(test, windows)), allow(dead_code))]
fn monitor_id_for_position(
    position: CharacterWindowPosition,
    monitors: &[DisplayMonitorSnapshot],
) -> Option<String> {
    monitors
        .iter()
        .find(|monitor| {
            position.x >= monitor.work_area.left
                && position.x < monitor.work_area.right
                && position.y >= monitor.work_area.top
                && position.y < monitor.work_area.bottom
        })
        .map(|monitor| monitor.option.id.clone())
}

#[cfg_attr(not(any(test, windows)), allow(dead_code))]
fn resolve_restored_window_position(
    target_monitor_id: Option<&str>,
    saved_position: Option<CharacterWindowPosition>,
    size: MascotWindowSize,
    monitors: &[DisplayMonitorSnapshot],
) -> Option<CharacterWindowPosition> {
    let target_monitor_id = target_monitor_id?;
    let target_monitor = monitors
        .iter()
        .find(|monitor| monitor.option.id == target_monitor_id)?;

    if let Some(saved_position) = saved_position {
        if monitor_id_for_position(saved_position, monitors).as_deref() == Some(target_monitor_id) {
            return Some(clamp_window_position_to_work_area(
                saved_position,
                size,
                target_monitor.work_area,
            ));
        }
    }

    let (x, y) = place_window_in_work_area(size, target_monitor.work_area);
    Some(CharacterWindowPosition { x, y })
}

#[cfg_attr(not(any(test, windows)), allow(dead_code))]
fn display_monitor_label(
    device_name: &str,
    friendly_name: Option<&str>,
    fallback_label: Option<&str>,
    is_primary: bool,
) -> String {
    let base = friendly_name
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .or_else(|| {
            fallback_label
                .map(str::trim)
                .filter(|name| !name.is_empty())
        })
        .unwrap_or(device_name);

    if is_primary {
        format!("{base} (Primary)")
    } else {
        base.to_string()
    }
}

#[cfg(windows)]
fn move_window_to_monitor_windows(
    state: &MascotWindowState,
    hwnd: windows_sys::Win32::Foundation::HWND,
    requested_monitor_id: Option<&str>,
) -> io::Result<MascotWindowPlacement> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER};

    let monitors = list_display_monitors_windows()?;
    let current_monitor_id = current_monitor_id_for_hwnd(hwnd)?;
    let current_position = current_window_position_for_hwnd(hwnd)?;

    if requested_monitor_id.is_none() {
        return Ok(MascotWindowPlacement {
            monitor_id: current_monitor_id,
            position: Some(current_position),
        });
    }

    let Some(target_monitor_id) = resolve_monitor_id(
        requested_monitor_id,
        current_monitor_id.as_deref(),
        &monitors,
    ) else {
        return Ok(MascotWindowPlacement {
            monitor_id: None,
            position: Some(current_position),
        });
    };

    if current_monitor_id.as_deref() == Some(target_monitor_id.as_str()) {
        return Ok(MascotWindowPlacement {
            monitor_id: Some(target_monitor_id),
            position: Some(current_position),
        });
    }

    let Some(target_monitor) = monitors
        .iter()
        .find(|monitor| monitor.option.id == target_monitor_id)
    else {
        return Ok(MascotWindowPlacement {
            monitor_id: current_monitor_id,
            position: Some(current_position),
        });
    };

    let (x, y) = place_window_in_work_area(state.size(), target_monitor.work_area);
    set_window_position_windows(
        state,
        hwnd,
        CharacterWindowPosition { x, y },
        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
    )?;

    Ok(MascotWindowPlacement {
        monitor_id: Some(target_monitor_id),
        position: Some(current_window_position_for_hwnd(hwnd)?),
    })
}

#[cfg(windows)]
fn move_window_to_position_windows(
    state: &MascotWindowState,
    hwnd: windows_sys::Win32::Foundation::HWND,
    requested_position: CharacterWindowPosition,
    requested_monitor_id: Option<&str>,
) -> io::Result<MascotWindowPlacement> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER};

    let monitors = list_display_monitors_windows()?;
    let current_monitor_id = current_monitor_id_for_hwnd(hwnd)?;
    let inferred_monitor_id = monitor_id_for_position(requested_position, &monitors);
    let resolved_monitor_id = resolve_monitor_id(
        requested_monitor_id.or(inferred_monitor_id.as_deref()),
        current_monitor_id.as_deref(),
        &monitors,
    );

    let Some(target_monitor_id) = resolved_monitor_id else {
        return Ok(MascotWindowPlacement {
            monitor_id: current_monitor_id,
            position: Some(current_window_position_for_hwnd(hwnd)?),
        });
    };

    let Some(target_monitor) = monitors
        .iter()
        .find(|monitor| monitor.option.id == target_monitor_id)
    else {
        return Ok(MascotWindowPlacement {
            monitor_id: current_monitor_id,
            position: Some(current_window_position_for_hwnd(hwnd)?),
        });
    };

    let clamped_position =
        clamp_requested_window_position(requested_position, state.size(), target_monitor.work_area);
    set_window_position_windows(
        state,
        hwnd,
        clamped_position,
        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
    )?;

    Ok(MascotWindowPlacement {
        monitor_id: Some(target_monitor_id),
        position: Some(current_window_position_for_hwnd(hwnd)?),
    })
}

#[cfg(windows)]
fn run_move_to_position_on_main_thread<R: Runtime>(
    app: &AppHandle<R>,
    requested_position: CharacterWindowPosition,
    requested_monitor_id: Option<String>,
) -> tauri::Result<MascotWindowPlacement> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let app_handle = app.clone();
    app.run_on_main_thread(move || {
        let result = (|| -> io::Result<MascotWindowPlacement> {
            let Some(state) = app_handle.try_state::<MascotWindowState>() else {
                return Ok(MascotWindowPlacement {
                    monitor_id: requested_monitor_id,
                    position: Some(requested_position),
                });
            };
            let window = *state
                .window
                .lock()
                .expect("mascot window state mutex poisoned");
            let Some(window) = window else {
                return Ok(MascotWindowPlacement {
                    monitor_id: requested_monitor_id,
                    position: Some(requested_position),
                });
            };

            move_window_to_position_windows(
                &state,
                window.hwnd as windows_sys::Win32::Foundation::HWND,
                requested_position,
                requested_monitor_id.as_deref(),
            )
        })();
        let _ = tx.send(result);
    })?;

    rx.recv()
        .map_err(|error| tauri::Error::from(io::Error::other(error.to_string())))?
        .map_err(tauri::Error::from)
}

#[cfg(windows)]
fn restore_window_placement_windows(
    state: &MascotWindowState,
    hwnd: windows_sys::Win32::Foundation::HWND,
    requested_monitor_id: Option<&str>,
    saved_position: Option<CharacterWindowPosition>,
) -> io::Result<MascotWindowPlacement> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER};

    let monitors = list_display_monitors_windows()?;
    let current_monitor_id = current_monitor_id_for_hwnd(hwnd)?;
    let preferred_monitor_id = requested_monitor_id
        .map(str::to_string)
        .or_else(|| {
            saved_position.and_then(|position| monitor_id_for_position(position, &monitors))
        })
        .or(current_monitor_id.clone());
    let resolved_monitor_id = resolve_monitor_id(
        preferred_monitor_id.as_deref(),
        current_monitor_id.as_deref(),
        &monitors,
    );
    let position = resolve_restored_window_position(
        resolved_monitor_id.as_deref(),
        saved_position,
        state.size(),
        &monitors,
    );

    if let Some(position) = position {
        set_window_position_windows(
            state,
            hwnd,
            position,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )?;
    }

    Ok(MascotWindowPlacement {
        monitor_id: resolved_monitor_id,
        position: current_window_position_for_hwnd(hwnd).ok(),
    })
}

#[cfg(windows)]
fn list_display_monitors_windows() -> io::Result<Vec<DisplayMonitorSnapshot>> {
    use windows_sys::Win32::Graphics::Gdi::EnumDisplayMonitors;

    unsafe extern "system" fn enumerate_monitors(
        monitor: windows_sys::Win32::Graphics::Gdi::HMONITOR,
        _hdc: windows_sys::Win32::Graphics::Gdi::HDC,
        _clip_rect: *mut windows_sys::Win32::Foundation::RECT,
        data: windows_sys::Win32::Foundation::LPARAM,
    ) -> i32 {
        let monitors = &mut *(data as *mut Vec<DisplayMonitorSnapshot>);
        if let Ok(snapshot) = monitor_snapshot_from_handle(monitor) {
            monitors.push(snapshot);
        }
        1
    }

    let mut monitors: Vec<DisplayMonitorSnapshot> = Vec::new();
    let result = unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(enumerate_monitors),
            (&mut monitors as *mut Vec<DisplayMonitorSnapshot>) as isize,
        )
    };
    if result == 0 {
        return Err(io::Error::last_os_error());
    }

    let friendly_names = display_monitor_friendly_names_windows().unwrap_or_default();
    for monitor in &mut monitors {
        monitor.option.label = display_monitor_label(
            &monitor.option.id,
            friendly_names.get(&monitor.option.id).map(String::as_str),
            None,
            monitor.is_primary,
        );
    }

    monitors.sort_by(
        |left: &DisplayMonitorSnapshot, right: &DisplayMonitorSnapshot| {
            left.option.id.cmp(&right.option.id)
        },
    );
    Ok(monitors)
}

#[cfg(windows)]
fn monitor_snapshot_from_handle(
    monitor: windows_sys::Win32::Graphics::Gdi::HMONITOR,
) -> io::Result<DisplayMonitorSnapshot> {
    use windows_sys::Win32::{
        Graphics::Gdi::{GetMonitorInfoW, MONITORINFOEXW},
        UI::WindowsAndMessaging::MONITORINFOF_PRIMARY,
    };

    let mut info: MONITORINFOEXW = unsafe { std::mem::zeroed() };
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

    let result = unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo) };
    if result == 0 {
        return Err(io::Error::last_os_error());
    }

    let device_name = wide_nul_terminated_to_string(&info.szDevice);
    let is_primary = (info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY) != 0;
    let label = display_monitor_label(&device_name, None, None, is_primary);

    Ok(DisplayMonitorSnapshot {
        option: DisplayMonitorOption {
            id: device_name,
            label,
            work_area_left: info.monitorInfo.rcWork.left,
            work_area_top: info.monitorInfo.rcWork.top,
            work_area_width: (info.monitorInfo.rcWork.right - info.monitorInfo.rcWork.left).max(0)
                as u32,
            work_area_height: (info.monitorInfo.rcWork.bottom - info.monitorInfo.rcWork.top).max(0)
                as u32,
        },
        work_area: MonitorWorkArea {
            left: info.monitorInfo.rcWork.left,
            top: info.monitorInfo.rcWork.top,
            right: info.monitorInfo.rcWork.right,
            bottom: info.monitorInfo.rcWork.bottom,
        },
        is_primary,
    })
}

#[cfg(windows)]
fn current_monitor_id_for_hwnd(
    hwnd: windows_sys::Win32::Foundation::HWND,
) -> io::Result<Option<String>> {
    use windows_sys::Win32::Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTONEAREST};

    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    if monitor.is_null() {
        return Ok(None);
    }

    Ok(Some(monitor_snapshot_from_handle(monitor)?.option.id))
}

#[cfg(windows)]
fn current_window_position_for_hwnd(
    hwnd: windows_sys::Win32::Foundation::HWND,
) -> io::Result<CharacterWindowPosition> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;

    let mut rect = RECT::default();
    let result = unsafe { GetWindowRect(hwnd, &mut rect) };
    if result == 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(CharacterWindowPosition {
        x: rect.left,
        y: rect.top,
    })
}

#[cfg(windows)]
fn set_window_position_windows(
    state: &MascotWindowState,
    hwnd: windows_sys::Win32::Foundation::HWND,
    position: CharacterWindowPosition,
    flags: u32,
) -> io::Result<()> {
    use windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos;

    state.set_monitor_move_suppressed(true);
    let result = unsafe {
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            position.x,
            position.y,
            0,
            0,
            flags,
        )
    };
    state.set_monitor_move_suppressed(false);

    if result == 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(())
}

#[cfg(windows)]
fn wide_nul_terminated_to_string(raw: &[u16]) -> String {
    let len = raw.iter().position(|ch| *ch == 0).unwrap_or(raw.len());
    String::from_utf16_lossy(&raw[..len])
}

#[cfg(windows)]
fn display_monitor_friendly_names_windows() -> io::Result<BTreeMap<String, String>> {
    use windows_sys::Win32::{
        Devices::Display::{
            DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
            DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
            DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_EMBEDDED,
            DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_LVDS,
            DISPLAYCONFIG_OUTPUT_TECHNOLOGY_UDI_EMBEDDED, DISPLAYCONFIG_PATH_INFO,
            DISPLAYCONFIG_SOURCE_DEVICE_NAME, DISPLAYCONFIG_TARGET_DEVICE_NAME,
            QDC_ONLY_ACTIVE_PATHS,
        },
        Foundation::WIN32_ERROR,
    };

    let mut path_count = 0_u32;
    let mut mode_count = 0_u32;
    let status = unsafe {
        GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
    };
    if status != 0 {
        return Err(io::Error::from_raw_os_error(status as i32));
    }

    let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
    let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];
    let status: WIN32_ERROR = unsafe {
        QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut path_count,
            paths.as_mut_ptr(),
            &mut mode_count,
            modes.as_mut_ptr(),
            std::ptr::null_mut(),
        )
    };
    if status != 0 {
        return Err(io::Error::from_raw_os_error(status as i32));
    }

    paths.truncate(path_count as usize);
    let mut friendly_names = BTreeMap::new();
    for path in paths {
        let mut source_name = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
        source_name.header.size = std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32;
        source_name.header.adapterId = path.sourceInfo.adapterId;
        source_name.header.id = path.sourceInfo.id;
        source_name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;

        let source_status = unsafe { DisplayConfigGetDeviceInfo(&mut source_name.header) };
        if source_status != 0 {
            continue;
        }

        let mut target_name = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
        target_name.header.size = std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
        target_name.header.adapterId = path.targetInfo.adapterId;
        target_name.header.id = path.targetInfo.id;
        target_name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;

        let target_status = unsafe { DisplayConfigGetDeviceInfo(&mut target_name.header) };
        if target_status != 0 {
            continue;
        }

        let source_device_name = wide_nul_terminated_to_string(&source_name.viewGdiDeviceName);
        let friendly_name = wide_nul_terminated_to_string(&target_name.monitorFriendlyDeviceName);
        let resolved_label = if !friendly_name.is_empty() {
            Some(friendly_name)
        } else {
            match target_name.outputTechnology {
                DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL
                | DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_EMBEDDED
                | DISPLAYCONFIG_OUTPUT_TECHNOLOGY_LVDS
                | DISPLAYCONFIG_OUTPUT_TECHNOLOGY_UDI_EMBEDDED => {
                    Some("Built-in Display".to_string())
                }
                _ => None,
            }
        };

        if !source_device_name.is_empty() {
            if let Some(resolved_label) = resolved_label {
                friendly_names
                    .entry(source_device_name)
                    .or_insert(resolved_label);
            }
        }
    }

    Ok(friendly_names)
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
fn attach_window_context<R: Runtime>(
    hwnd: windows_sys::Win32::Foundation::HWND,
    app: &AppHandle<R>,
) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SetWindowLongPtrW, GWLP_USERDATA};

    let app_handle = app.clone();
    let context = Box::new(NativeMascotWindowContext {
        on_window_pos_changed: Box::new(move |monitor_id, position| {
            if let Some(state) = app_handle.try_state::<MascotWindowState>() {
                if state.is_monitor_move_suppressed() {
                    return;
                }
            }

            let _ = crate::app_settings::sync_character_window_placement_from_window(
                &app_handle,
                monitor_id,
                position,
            );
        }),
    });

    unsafe {
        let _ = SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(context) as isize);
    }
}

#[cfg(windows)]
unsafe fn window_context(
    hwnd: windows_sys::Win32::Foundation::HWND,
) -> Option<&'static NativeMascotWindowContext> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, GWLP_USERDATA};

    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut NativeMascotWindowContext;
    ptr.as_ref()
}

#[cfg(windows)]
unsafe extern "system" fn mascot_window_proc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DefWindowProcA, GetWindowLongPtrW, SendMessageA, SetWindowLongPtrW, GWLP_USERDATA,
        HTCAPTION, WM_LBUTTONDOWN, WM_NCDESTROY, WM_NCLBUTTONDOWN, WM_WINDOWPOSCHANGED,
    };

    if message == WM_LBUTTONDOWN {
        let _ = SendMessageA(hwnd, WM_NCLBUTTONDOWN, HTCAPTION as usize, 0);
        return 0;
    }

    if message == WM_WINDOWPOSCHANGED {
        if let Some(context) = window_context(hwnd) {
            let monitor_id = current_monitor_id_for_hwnd(hwnd).ok().flatten();
            let position = current_window_position_for_hwnd(hwnd).ok();
            (context.on_window_pos_changed)(monitor_id, position);
        }
    }

    if message == WM_NCDESTROY {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut NativeMascotWindowContext;
        if !ptr.is_null() {
            let _ = Box::from_raw(ptr);
            let _ = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        }
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
