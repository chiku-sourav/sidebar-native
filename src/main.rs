#![windows_subsystem = "windows"]
#![allow(unused_imports, dead_code, unused_must_use)]

mod config;
mod logger;
mod telemetry;
mod window;

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use windows::core::w;
use windows::Win32::Foundation::{
    GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
};
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};
use windows::Win32::Graphics::Gdi::{BeginPaint, EndPaint, InvalidateRect, PAINTSTRUCT};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::HiDpi::{
    GetDpiForSystem, GetDpiForWindow, SetProcessDpiAwarenessContext,
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, ReleaseCapture, SetFocus, UnregisterHotKey, MOD_ALT, MOD_CONTROL, VK_S,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, GetMessageW, GetWindowLongW,
    GetWindowRect, LoadCursorW, PostQuitMessage, RegisterClassExW, SendMessageW, SetCursor,
    SetForegroundWindow, SetTimer, SetWindowLongW, SetWindowPos, ShowWindow, TranslateMessage,
    CS_HREDRAW, CS_VREDRAW, GWL_EXSTYLE, HMENU, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCLIENT,
    HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT, HWND_NOTOPMOST, HWND_TOPMOST, IDC_ARROW,
    MINMAXINFO, MSG, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_HIDE, SW_SHOW,
    SW_SHOWNOACTIVATE, WM_ACTIVATE, WM_DESTROY, WM_DPICHANGED, WM_ERASEBKGND, WM_GETMINMAXINFO,
    WM_HOTKEY, WM_KILLFOCUS, WM_LBUTTONDOWN, WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_NCHITTEST, WM_NCLBUTTONDOWN, WM_PAINT, WM_SETCURSOR, WM_SETFOCUS, WM_SETTINGCHANGE, WM_SIZE,
    WM_TIMER, WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
    WS_THICKFRAME,
};

use config::{
    AppConfig, AppTheme, BackdropEffect, DateFormat, FontSize, ProcessSortBy, TemperatureUnit,
    WindowWidthPreset,
};
use logger::Logger;
use telemetry::TelemetryEngine;
use window::tray::*;
use window::{
    AppBarManager, BackdropManager, FullscreenDetector, StartupManager, SystemTray, UIRenderer,
};

const DWMWA_WINDOW_CORNER_PREFERENCE: DWMWINDOWATTRIBUTE = DWMWINDOWATTRIBUTE(33);
const DWMWCP_ROUND: i32 = 2;

const BASE_WIDTH: i32 = 380;
const BASE_HEIGHT: i32 = 620;

static APP_STATE: OnceLock<AppState> = OnceLock::new();

struct AppState {
    config: Mutex<AppConfig>,
    telemetry: TelemetryEngine,
    renderer: Mutex<UIRenderer>,
    tray: Mutex<SystemTray>,
    is_visible: AtomicBool,
    is_dark_mode: AtomicBool,
    scroll_offset_y: AtomicI32,
    max_scroll_y: AtomicI32,
    dpi: AtomicU32,
    win_width: AtomicI32,
    win_height: AtomicI32,
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // Log non-frequent messages
    if msg != 0x0020 && msg != 0x0200 && msg != 0x0113 && msg != 0x0084 {
        log_debug!("wnd_proc: msg=0x{:04X}, wparam=0x{:X}", msg, wparam.0);
    }

    match msg {
        WM_NCHITTEST => {
            let px = (lparam.0 & 0xFFFF) as i16 as i32;
            let py = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;

            let mut rect = RECT::default();
            let _ = GetWindowRect(hwnd, &mut rect);
            let b = 8; // 8px resizable border hit area

            let on_left = px >= rect.left && px < rect.left + b;
            let on_right = px <= rect.right && px > rect.right - b;
            let on_top = py >= rect.top && py < rect.top + b;
            let on_bottom = py <= rect.bottom && py > rect.bottom - b;

            if on_top && on_left {
                LRESULT(HTTOPLEFT as isize)
            } else if on_top && on_right {
                LRESULT(HTTOPRIGHT as isize)
            } else if on_bottom && on_left {
                LRESULT(HTBOTTOMLEFT as isize)
            } else if on_bottom && on_right {
                LRESULT(HTBOTTOMRIGHT as isize)
            } else if on_left {
                LRESULT(HTLEFT as isize)
            } else if on_right {
                LRESULT(HTRIGHT as isize)
            } else if on_top {
                LRESULT(HTTOP as isize)
            } else if on_bottom {
                LRESULT(HTBOTTOM as isize)
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }

        WM_GETMINMAXINFO => {
            let minmax = &mut *(lparam.0 as *mut MINMAXINFO);
            minmax.ptMinTrackSize.x = 280;
            minmax.ptMinTrackSize.y = 200;
            LRESULT(0)
        }

        WM_SIZE => {
            let w = (lparam.0 & 0xFFFF) as i32;
            let h = ((lparam.0 >> 16) & 0xFFFF) as i32;
            if w > 0 && h > 0 {
                if let Some(state) = APP_STATE.get() {
                    state.win_width.store(w, Ordering::Relaxed);
                    state.win_height.store(h, Ordering::Relaxed);
                    let mut cfg = state.config.lock().unwrap();
                    cfg.sidebar_width = w;
                    let _ = cfg.save();
                }
                let _ = InvalidateRect(hwnd, None, false);
            }
            LRESULT(0)
        }

        WM_ERASEBKGND => {
            // Handled: Double-buffering takes care of painting background
            LRESULT(1)
        }

        WM_SETCURSOR => {
            let cursor = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
            SetCursor(cursor);
            LRESULT(1)
        }

        WM_PAINT => {
            let start = Instant::now();
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);

            if let Some(state) = APP_STATE.get() {
                let config = state.config.lock().unwrap().clone();
                let snapshot = state.telemetry.snapshot().read().unwrap().clone();
                let is_dark = state.is_dark_mode.load(Ordering::Relaxed);
                let scroll_y = state.scroll_offset_y.load(Ordering::Relaxed);
                let cur_w = state.win_width.load(Ordering::Relaxed);
                let cur_h = state.win_height.load(Ordering::Relaxed);

                let total_content_h = {
                    let renderer_guard = state.renderer.lock().unwrap();
                    renderer_guard.render(hdc, cur_w, cur_h, scroll_y, &snapshot, &config, is_dark)
                };

                let visible_h = cur_h - 38;
                let max_s = (total_content_h - visible_h).max(0);
                state.max_scroll_y.store(max_s, Ordering::Relaxed);
            }

            EndPaint(hwnd, &mut ps);
            let elapsed_us = start.elapsed().as_micros();
            log_debug!("WM_PAINT completed in {} µs.", elapsed_us);
            LRESULT(0)
        }

        WM_DPICHANGED => {
            let new_dpi = (wparam.0 & 0xFFFF) as u32;
            log_info!("WM_DPICHANGED received -> new DPI = {}", new_dpi);
            if let Some(state) = APP_STATE.get() {
                let config = state.config.lock().unwrap().clone();
                state.dpi.store(new_dpi, Ordering::Relaxed);
                if let Ok(mut renderer) = state.renderer.lock() {
                    renderer.update_fonts(new_dpi, config.font_size.scale());
                }

                let dpi_scale = (new_dpi as f32 / 96.0).max(1.0);
                let font_scale = config.font_size.scale();
                let new_w = (BASE_WIDTH as f32 * dpi_scale * (1.0 + (font_scale - 1.0) * 0.35))
                    .round() as i32;
                let new_h = (BASE_HEIGHT as f32 * dpi_scale * (1.0 + (font_scale - 1.0) * 0.25))
                    .round() as i32;
                state.win_width.store(new_w, Ordering::Relaxed);
                state.win_height.store(new_h, Ordering::Relaxed);

                let suggested_rect = *(lparam.0 as *const RECT);
                let _ = SetWindowPos(
                    hwnd,
                    HWND(0 as *mut _),
                    suggested_rect.left,
                    suggested_rect.top,
                    new_w,
                    new_h,
                    SWP_NOACTIVATE,
                );
                let _ = InvalidateRect(hwnd, None, false);
            }
            LRESULT(0)
        }

        WM_MOUSEWHEEL => {
            let delta = ((wparam.0 >> 16) as i16) as i32;
            if let Some(state) = APP_STATE.get() {
                let current = state.scroll_offset_y.load(Ordering::Relaxed);
                let max_s = state.max_scroll_y.load(Ordering::Relaxed);
                let step = 45; // Smooth scroll amount

                let new_scroll = if delta > 0 {
                    (current - step).max(0)
                } else {
                    (current + step).min(max_s)
                };

                if new_scroll != current {
                    state.scroll_offset_y.store(new_scroll, Ordering::Relaxed);
                    let _ = InvalidateRect(hwnd, None, false);
                }
            }
            LRESULT(0)
        }

        WM_MOUSEACTIVATE => {
            LRESULT(1) // MA_ACTIVATE
        }

        WM_LBUTTONDOWN => {
            let x = (lparam.0 & 0xFFFF) as i16 as i32;
            let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
            log_debug!("WM_LBUTTONDOWN at x={}, y={}", x, y);

            let cur_w = if let Some(state) = APP_STATE.get() {
                state.win_width.load(Ordering::Relaxed)
            } else {
                BASE_WIDTH
            };

            // Click close button [✕] at top right (x > cur_w - 45 && y < 38)
            if x > (cur_w - 45) && y < 38 {
                log_info!("User clicked Close [✕] button.");
                hide_flyout(hwnd);
                return LRESULT(0);
            }

            // Click sticky header area (y < 38) to drag window freely
            if y < 38 {
                log_debug!("Initiating window drag from header.");
                let _ = ReleaseCapture();
                let _ = SendMessageW(hwnd, WM_NCLBUTTONDOWN, WPARAM(2), LPARAM(0)); // 2 = HTCAPTION
                return LRESULT(0);
            }

            LRESULT(0)
        }

        WM_TIMER => {
            if let Some(state) = APP_STATE.get() {
                let config = state.config.lock().unwrap().clone();

                // Check fullscreen auto-pause
                if config.auto_pause_fullscreen {
                    let is_fs = FullscreenDetector::is_foreground_fullscreen();
                    state.telemetry.set_paused(is_fs);
                }

                let snapshot = state.telemetry.snapshot().read().unwrap().clone();

                // Update live taskbar tray RAM % badge without blocking
                if let Ok(mut tray_guard) = state.tray.try_lock() {
                    tray_guard.update_pill_badge(snapshot.ram.usage_percentage as u8);
                }

                // If flyout is visible, redraw smoothly
                if state.is_visible.load(Ordering::Relaxed) {
                    let _ = InvalidateRect(hwnd, None, false);
                }
            }
            LRESULT(0)
        }

        WM_HOTKEY => {
            log_info!("Global hotkey Ctrl+Alt+S pressed.");
            toggle_visibility(hwnd);
            LRESULT(0)
        }

        WM_TRAYICON => {
            let event = (lparam.0 & 0xFFFF) as u32;
            match event {
                0x0202 => {
                    // WM_LBUTTONUP
                    log_info!("Tray icon clicked (Left Click) -> Toggling flyout.");
                    toggle_visibility(hwnd);
                }
                0x0205 => {
                    // WM_RBUTTONUP
                    log_info!("Tray icon clicked (Right Click) -> Opening context popup menu.");
                    if let Some(state) = APP_STATE.get() {
                        let action = {
                            if let Ok(tray) = state.tray.lock() {
                                let config = state.config.lock().unwrap().clone();
                                tray.show_context_menu(&config)
                            } else {
                                0
                            }
                        };
                        match action {
                            ID_TRAY_TOGGLE => {
                                log_info!("Tray Menu -> Show / Hide Flyout selected.");
                                toggle_visibility(hwnd);
                            }
                            ID_THEME_AUTO => {
                                log_info!("Theme changed -> Auto (Windows Sync)");
                                let mut cfg = state.config.lock().unwrap();
                                cfg.theme = AppTheme::Auto;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_THEME_DARK => {
                                log_info!("Theme changed -> Dark Slate");
                                let mut cfg = state.config.lock().unwrap();
                                cfg.theme = AppTheme::DarkSlate;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_THEME_LIGHT => {
                                log_info!("Theme changed -> Light Clean");
                                let mut cfg = state.config.lock().unwrap();
                                cfg.theme = AppTheme::LightMode;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_THEME_OLED => {
                                log_info!("Theme changed -> OLED Black");
                                let mut cfg = state.config.lock().unwrap();
                                cfg.theme = AppTheme::OledBlack;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_THEME_NORD => {
                                log_info!("Theme changed -> Nord Arctic");
                                let mut cfg = state.config.lock().unwrap();
                                cfg.theme = AppTheme::Nord;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_THEME_CYBERPUNK => {
                                log_info!("Theme changed -> Cyberpunk Neon");
                                let mut cfg = state.config.lock().unwrap();
                                cfg.theme = AppTheme::Cyberpunk;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_FONT_SMALL | ID_FONT_MEDIUM | ID_FONT_LARGE | ID_FONT_XLARGE
                            | ID_FONT_HUGE => {
                                let new_font_size = match action {
                                    ID_FONT_SMALL => FontSize::Small,
                                    ID_FONT_MEDIUM => FontSize::Medium,
                                    ID_FONT_LARGE => FontSize::Large,
                                    ID_FONT_XLARGE => FontSize::ExtraLarge,
                                    ID_FONT_HUGE => FontSize::Huge,
                                    _ => FontSize::Large,
                                };
                                log_info!("Font size changed -> {:?}", new_font_size);
                                let mut cfg = state.config.lock().unwrap();
                                cfg.font_size = new_font_size;
                                let _ = cfg.save();
                                let font_scale = cfg.font_size.scale();
                                drop(cfg);

                                let cur_dpi = state.dpi.load(Ordering::Relaxed);
                                if let Ok(mut r) = state.renderer.lock() {
                                    r.update_fonts(cur_dpi, font_scale);
                                }
                                let dpi_scale = (cur_dpi as f32 / 96.0).max(1.0);
                                let new_w = (BASE_WIDTH as f32
                                    * dpi_scale
                                    * (1.0 + (font_scale - 1.0) * 0.35))
                                    .round() as i32;
                                let new_h = (BASE_HEIGHT as f32
                                    * dpi_scale
                                    * (1.0 + (font_scale - 1.0) * 0.25))
                                    .round() as i32;
                                state.win_width.store(new_w, Ordering::Relaxed);
                                state.win_height.store(new_h, Ordering::Relaxed);

                                let rect = AppBarManager::get_flyout_rect(new_w, new_h);
                                let _ = SetWindowPos(
                                    hwnd,
                                    HWND(0 as *mut _),
                                    rect.left,
                                    rect.top,
                                    new_w,
                                    new_h,
                                    SWP_NOACTIVATE,
                                );
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_BACKDROP_MICA => {
                                log_info!("Backdrop changed -> Mica");
                                let mut cfg = state.config.lock().unwrap();
                                cfg.backdrop = BackdropEffect::Mica;
                                let _ = cfg.save();
                                let is_dark = state.is_dark_mode.load(Ordering::Relaxed);
                                BackdropManager::apply_backdrop(
                                    hwnd,
                                    BackdropEffect::Mica,
                                    is_dark,
                                );
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_BACKDROP_ACRYLIC => {
                                log_info!("Backdrop changed -> Acrylic");
                                let mut cfg = state.config.lock().unwrap();
                                cfg.backdrop = BackdropEffect::Acrylic;
                                let _ = cfg.save();
                                let is_dark = state.is_dark_mode.load(Ordering::Relaxed);
                                BackdropManager::apply_backdrop(
                                    hwnd,
                                    BackdropEffect::Acrylic,
                                    is_dark,
                                );
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_BACKDROP_MICAALT => {
                                log_info!("Backdrop changed -> Mica Alt");
                                let mut cfg = state.config.lock().unwrap();
                                cfg.backdrop = BackdropEffect::MicaAlt;
                                let _ = cfg.save();
                                let is_dark = state.is_dark_mode.load(Ordering::Relaxed);
                                BackdropManager::apply_backdrop(
                                    hwnd,
                                    BackdropEffect::MicaAlt,
                                    is_dark,
                                );
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_BACKDROP_NONE => {
                                log_info!("Backdrop changed -> None / Solid");
                                let mut cfg = state.config.lock().unwrap();
                                cfg.backdrop = BackdropEffect::None;
                                let _ = cfg.save();
                                let is_dark = state.is_dark_mode.load(Ordering::Relaxed);
                                BackdropManager::apply_backdrop(
                                    hwnd,
                                    BackdropEffect::None,
                                    is_dark,
                                );
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_WIDTH_COMPACT => {
                                log_info!("Window Width Preset -> Compact (350px)");
                                let mut cfg = state.config.lock().unwrap();
                                cfg.width_preset = WindowWidthPreset::Compact;
                                cfg.sidebar_width = WindowWidthPreset::Compact.base_width();
                                let _ = cfg.save();
                                let new_w = cfg.sidebar_width;
                                state.win_width.store(new_w, Ordering::Relaxed);
                                let cur_h = state.win_height.load(Ordering::Relaxed);
                                drop(cfg);
                                let rect = AppBarManager::get_flyout_rect(new_w, cur_h);
                                let _ = SetWindowPos(hwnd, HWND(0 as *mut _), rect.left, rect.top, new_w, cur_h, SWP_NOACTIVATE);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_WIDTH_STANDARD => {
                                log_info!("Window Width Preset -> Standard (410px)");
                                let mut cfg = state.config.lock().unwrap();
                                cfg.width_preset = WindowWidthPreset::Standard;
                                cfg.sidebar_width = WindowWidthPreset::Standard.base_width();
                                let _ = cfg.save();
                                let new_w = cfg.sidebar_width;
                                state.win_width.store(new_w, Ordering::Relaxed);
                                let cur_h = state.win_height.load(Ordering::Relaxed);
                                drop(cfg);
                                let rect = AppBarManager::get_flyout_rect(new_w, cur_h);
                                let _ = SetWindowPos(hwnd, HWND(0 as *mut _), rect.left, rect.top, new_w, cur_h, SWP_NOACTIVATE);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_WIDTH_WIDE => {
                                log_info!("Window Width Preset -> Wide (490px)");
                                let mut cfg = state.config.lock().unwrap();
                                cfg.width_preset = WindowWidthPreset::Wide;
                                cfg.sidebar_width = WindowWidthPreset::Wide.base_width();
                                let _ = cfg.save();
                                let new_w = cfg.sidebar_width;
                                state.win_width.store(new_w, Ordering::Relaxed);
                                let cur_h = state.win_height.load(Ordering::Relaxed);
                                drop(cfg);
                                let rect = AppBarManager::get_flyout_rect(new_w, cur_h);
                                let _ = SetWindowPos(hwnd, HWND(0 as *mut _), rect.left, rect.top, new_w, cur_h, SWP_NOACTIVATE);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_WIDTH_ULTRAWIDE => {
                                log_info!("Window Width Preset -> UltraWide (580px)");
                                let mut cfg = state.config.lock().unwrap();
                                cfg.width_preset = WindowWidthPreset::UltraWide;
                                cfg.sidebar_width = WindowWidthPreset::UltraWide.base_width();
                                let _ = cfg.save();
                                let new_w = cfg.sidebar_width;
                                state.win_width.store(new_w, Ordering::Relaxed);
                                let cur_h = state.win_height.load(Ordering::Relaxed);
                                drop(cfg);
                                let rect = AppBarManager::get_flyout_rect(new_w, cur_h);
                                let _ = SetWindowPos(hwnd, HWND(0 as *mut _), rect.left, rect.top, new_w, cur_h, SWP_NOACTIVATE);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_PROC_SORT_CPU => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.sort_processes_by = ProcessSortBy::Cpu;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_PROC_SORT_RAM => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.sort_processes_by = ProcessSortBy::Memory;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_PROC_SORT_DISK => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.sort_processes_by = ProcessSortBy::Disk;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_POLL_500MS => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.poll_interval_ms = 500;
                                let _ = cfg.save();
                            }
                            ID_POLL_1000MS => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.poll_interval_ms = 1000;
                                let _ = cfg.save();
                            }
                            ID_POLL_2000MS => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.poll_interval_ms = 2000;
                                let _ = cfg.save();
                            }
                            ID_POLL_3000MS => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.poll_interval_ms = 3000;
                                let _ = cfg.save();
                            }
                            ID_POLL_5000MS => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.poll_interval_ms = 5000;
                                let _ = cfg.save();
                            }
                            ID_CLOCK_TOGGLE => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.show_clock = !cfg.show_clock;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_CLOCK_24HR => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.clock_24hr = !cfg.clock_24hr;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_MACHINE_NAME => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.show_machine_name = !cfg.show_machine_name;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_DATE_DISABLED => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.date_format = DateFormat::Disabled;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_DATE_SHORT => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.date_format = DateFormat::Short;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_DATE_NORMAL => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.date_format = DateFormat::Normal;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_DATE_LONG => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.date_format = DateFormat::Long;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_TEMP_CELSIUS => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.temperature_unit = TemperatureUnit::Celsius;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_TEMP_FAHRENHEIT => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.temperature_unit = TemperatureUnit::Fahrenheit;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_UNIT_GHZ => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.use_ghz = true;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_UNIT_MHZ => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.use_ghz = false;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_SPEED_BYTES => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.use_bytes = true;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_SPEED_BITS => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.use_bytes = false;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_TOGGLE_CORE_LOADS => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.show_core_loads = !cfg.show_core_loads;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_TOGGLE_CPU => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.show_cpu = !cfg.show_cpu;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_TOGGLE_GPU => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.show_gpu = !cfg.show_gpu;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_TOGGLE_AUDIO => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.show_audio = !cfg.show_audio;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_TOGGLE_RAM => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.show_ram = !cfg.show_ram;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_TOGGLE_STORAGE => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.show_storage = !cfg.show_storage;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_TOGGLE_NETWORK => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.show_network = !cfg.show_network;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_TOGGLE_PROCESSES => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.show_processes = !cfg.show_processes;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_TOGGLE_VM => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.show_virtual_memory = !cfg.show_virtual_memory;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_TOGGLE_BATTERY => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.show_battery = !cfg.show_battery;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_TOGGLE_SYSTEM => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.show_system_overview = !cfg.show_system_overview;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_TOGGLE_SENSORS_CARD => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.show_sensors_card = !cfg.show_sensors_card;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_TOGGLE_DISABLED_HARDWARE => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.show_disabled_hardware = !cfg.show_disabled_hardware;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_TOGGLE_ALL_GPUS => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.show_all_gpus = !cfg.show_all_gpus;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_TOGGLE_GPU_SHARED => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.show_gpu_shared_memory = !cfg.show_gpu_shared_memory;
                                let _ = cfg.save();
                                drop(cfg);
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                            ID_STARTUP_TOGGLE => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.run_at_startup = !cfg.run_at_startup;
                                let _ = StartupManager::set_run_at_startup(cfg.run_at_startup);
                                let _ = cfg.save();
                            }
                            ID_AUTOPAUSE_TOGGLE => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.auto_pause_fullscreen = !cfg.auto_pause_fullscreen;
                                let _ = cfg.save();
                            }
                            ID_TOPMOST_TOGGLE => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.stay_on_top = !cfg.stay_on_top;
                                let _ = cfg.save();
                                let target_hwnd = if cfg.stay_on_top {
                                    HWND_TOPMOST
                                } else {
                                    HWND_NOTOPMOST
                                };
                                let _ = SetWindowPos(
                                    hwnd,
                                    target_hwnd,
                                    0,
                                    0,
                                    0,
                                    0,
                                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                                );
                            }
                            ID_CLICKTHROUGH_TOGGLE => {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.click_through = !cfg.click_through;
                                let _ = cfg.save();
                                let current_ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
                                let new_ex = if cfg.click_through {
                                    current_ex | (WS_EX_TRANSPARENT.0 as i32)
                                } else {
                                    current_ex & !(WS_EX_TRANSPARENT.0 as i32)
                                };
                                SetWindowLongW(hwnd, GWL_EXSTYLE, new_ex);
                            }
                            ID_TRAY_OPEN_CONFIG => {
                                log_info!("Tray Menu -> Open Config File selected.");
                                let cfg_path = AppConfig::config_path();
                                let _ = std::process::Command::new("notepad.exe")
                                    .arg(cfg_path)
                                    .spawn();
                            }
                            ID_TRAY_LOGS => {
                                log_info!("Tray Menu -> Open Debug Log selected.");
                                let log_path = Logger::get_log_path();
                                let _ = std::process::Command::new("notepad.exe")
                                    .arg(log_path)
                                    .spawn();
                            }
                            ID_TRAY_ABOUT => {
                                log_info!("Tray Menu -> About Diagnostics selected.");
                                use windows::Win32::UI::WindowsAndMessaging::{
                                    MessageBoxW, MB_ICONINFORMATION, MB_OK,
                                };
                                MessageBoxW(
                                    hwnd,
                                    w!("Sidebar Diagnostics Native (Rust)\n\nUltra-low resource Windows 11 Diagnostics Flyout.\nIdle Memory: < 8 MB RAM\nIdle CPU: < 0.1%\n\nFeatures full GPU, CPU, Audio, RAM, Disk, Network, Thermals & Process monitoring with smooth scrolling and high-DPI scaling.\n\nDebug logs: %APPDATA%\\SidebarNative\\sidebar.log"),
                                    w!("About Sidebar Diagnostics"),
                                    MB_OK | MB_ICONINFORMATION,
                                );
                            }
                            ID_TRAY_EXIT => {
                                log_info!("Tray Menu -> Exit Application selected. Exiting.");
                                let _ = ShowWindow(hwnd, SW_HIDE);
                                PostQuitMessage(0);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
            LRESULT(0)
        }

        WM_SETFOCUS => {
            log_debug!("WM_SETFOCUS received.");
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }

        WM_KILLFOCUS => {
            log_debug!("WM_KILLFOCUS received.");
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }

        WM_SETTINGCHANGE => {
            log_info!("WM_SETTINGCHANGE received -> Syncing Windows Dark/Light theme.");
            if let Some(state) = APP_STATE.get() {
                let is_dark = BackdropManager::is_system_dark_mode();
                state.is_dark_mode.store(is_dark, Ordering::Relaxed);
                let config = state.config.lock().unwrap().clone();
                BackdropManager::apply_backdrop(hwnd, config.backdrop, is_dark);
                let _ = InvalidateRect(hwnd, None, false);
            }
            LRESULT(0)
        }

        WM_DESTROY => {
            log_info!("WM_DESTROY received -> Posting quit message.");
            PostQuitMessage(0);
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn toggle_visibility(hwnd: HWND) {
    if let Some(state) = APP_STATE.get() {
        let currently_visible = state.is_visible.load(Ordering::Relaxed);
        log_info!("toggle_visibility: currently_visible={}", currently_visible);
        if currently_visible {
            hide_flyout(hwnd);
        } else {
            show_flyout(hwnd);
        }
    }
}

fn show_flyout(hwnd: HWND) {
    if let Some(state) = APP_STATE.get() {
        log_info!("show_flyout: positioning window above tray and showing.");
        state.is_visible.store(true, Ordering::Relaxed);
        state.telemetry.set_paused(false);

        let cur_w = state.win_width.load(Ordering::Relaxed);
        let cur_h = state.win_height.load(Ordering::Relaxed);

        unsafe {
            let rect = AppBarManager::get_flyout_rect(cur_w, cur_h);
            let _ = SetWindowPos(
                hwnd,
                HWND(0 as *mut _),
                rect.left,
                rect.top,
                cur_w,
                cur_h,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            let _ = InvalidateRect(hwnd, None, false);
        }
    }
}

fn hide_flyout(hwnd: HWND) {
    if let Some(state) = APP_STATE.get() {
        log_info!("hide_flyout: hiding window.");
        state.is_visible.store(false, Ordering::Relaxed);
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }
}

fn main() {
    // 1. Initialize logging system
    if let Err(e) = Logger::init() {
        eprintln!("Failed to initialize logger: {:?}", e);
    }

    unsafe {
        // Set Per-Monitor High-DPI Awareness V2
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        log_info!("Checking single instance named mutex...");
        let mutex_name = w!("Local\\SidebarDiagnosticsNativeMutex");
        let _h_mutex = CreateMutexW(None, true, mutex_name);

        log_info!("Loading application configuration...");
        let config = AppConfig::load();
        let is_dark = BackdropManager::is_system_dark_mode();
        log_info!("Windows system theme detected: is_dark={}", is_dark);

        let init_dpi = GetDpiForSystem();
        log_info!("System DPI detected: {}", init_dpi);
        let dpi_scale = (init_dpi as f32 / 96.0).max(1.0);
        let font_scale = config.font_size.scale();
        let win_w =
            (BASE_WIDTH as f32 * dpi_scale * (1.0 + (font_scale - 1.0) * 0.35)).round() as i32;
        let win_h =
            (BASE_HEIGHT as f32 * dpi_scale * (1.0 + (font_scale - 1.0) * 0.25)).round() as i32;

        log_info!("Initializing hardware telemetry engine with GPU & Thermals monitoring...");
        let telemetry = TelemetryEngine::new();
        telemetry.start(config.clone());

        log_info!("Registering Win32 window class...");
        let h_instance = GetModuleHandleW(None).unwrap_or_default();
        let class_name = w!("SidebarDiagnosticsFlyoutWindowClass");

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: HINSTANCE(h_instance.0),
            lpszClassName: class_name,
            ..Default::default()
        };

        RegisterClassExW(&wc);

        let rect = AppBarManager::get_flyout_rect(win_w, win_h);
        log_info!(
            "Calculated flyout window rect: left={}, top={}, right={}, bottom={}",
            rect.left,
            rect.top,
            rect.right,
            rect.bottom
        );

        let mut ex_style = WS_EX_TOOLWINDOW;
        if config.stay_on_top {
            ex_style |= WS_EX_TOPMOST;
        }
        if config.click_through {
            ex_style |= WS_EX_TRANSPARENT;
        }

        let hwnd_res = CreateWindowExW(
            ex_style,
            class_name,
            w!("Sidebar Diagnostics Flyout"),
            WS_POPUP,
            rect.left,
            rect.top,
            win_w,
            win_h,
            None,
            HMENU(0 as *mut _),
            HINSTANCE(h_instance.0),
            None,
        );

        let hwnd = match hwnd_res {
            Ok(h) => {
                log_info!("Successfully created flyout window with HWND: {:?}", h.0);
                h
            }
            Err(e) => {
                log_error!("Failed to create window: {:?}", e);
                return;
            }
        };

        log_info!("Configuring DWM rounded corners and backdrop...");
        let corner_pref = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &corner_pref as *const _ as *const _,
            std::mem::size_of::<i32>() as u32,
        );
        BackdropManager::apply_backdrop(hwnd, config.backdrop, is_dark);

        log_info!("Creating system tray icon instance...");
        let tray = SystemTray::new(hwnd);

        let renderer = UIRenderer::new(init_dpi, config.font_size.scale());

        let start_visible = !config.initially_hidden;

        let state = AppState {
            config: Mutex::new(config.clone()),
            telemetry,
            renderer: Mutex::new(renderer),
            tray: Mutex::new(tray),
            is_visible: AtomicBool::new(start_visible),
            is_dark_mode: AtomicBool::new(is_dark),
            scroll_offset_y: AtomicI32::new(0),
            max_scroll_y: AtomicI32::new(0),
            dpi: AtomicU32::new(init_dpi),
            win_width: AtomicI32::new(win_w),
            win_height: AtomicI32::new(win_h),
        };
        let _ = APP_STATE.set(state);

        log_info!("Registering global hotkey Ctrl+Alt+S...");
        let _ = RegisterHotKey(hwnd, 1, MOD_CONTROL | MOD_ALT, VK_S.0 as u32);
        let _ = SetTimer(hwnd, 1, 1000, None);

        if start_visible {
            log_info!("Step 1: Calling ShowWindow(SW_SHOWNOACTIVATE)...");
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            log_info!("Step 2: Calling InvalidateRect...");
            let _ = InvalidateRect(hwnd, None, false);
        } else {
            log_info!("Starting minimized/hidden to system tray.");
            let _ = ShowWindow(hwnd, SW_HIDE);
        }

        log_info!("Step 3: Spawning background tray thread...");
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if let Some(state) = APP_STATE.get() {
                if let Ok(tray) = state.tray.lock() {
                    tray.register();
                }
            }
        });

        log_info!("Step 4: Entering Win32 message loop pump.");
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        log_info!("Win32 message loop exited cleanly. Cleaning up hotkeys and handles.");
        let _ = UnregisterHotKey(hwnd, 1);
        log_info!("Sidebar Diagnostics Native terminated cleanly.");
    }
}
