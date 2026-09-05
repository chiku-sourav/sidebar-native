use std::sync::atomic::Ordering;
use std::time::Instant;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{BeginPaint, EndPaint, InvalidateRect, PAINTSTRUCT};
use windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
use windows::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, GetWindowRect, LoadCursorW, PostQuitMessage, SendMessageW, SetCursor,
    SetWindowPos, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT,
    HTTOPRIGHT, IDC_ARROW, MINMAXINFO, SWP_NOACTIVATE, WM_DESTROY, WM_DPICHANGED, WM_ERASEBKGND,
    WM_GETMINMAXINFO, WM_HOTKEY, WM_KILLFOCUS, WM_LBUTTONDOWN, WM_MOUSEACTIVATE, WM_MOUSEMOVE,
    WM_MOUSEWHEEL, WM_NCHITTEST, WM_NCLBUTTONDOWN, WM_PAINT, WM_SETCURSOR, WM_SETFOCUS,
    WM_SETTINGCHANGE, WM_SIZE, WM_TIMER,
};

use super::super::appbar::AppBarManager;
use super::super::backdrop::BackdropManager;
use super::super::flyout::{hide_flyout, toggle_visibility};
use super::super::fullscreen::FullscreenDetector;
use super::super::state::{get_app_state, BASE_HEIGHT, BASE_WIDTH};
use super::tray_dispatch::handle_tray_menu_action;
use crate::window::tray::WM_TRAYICON;
use crate::{log_debug, log_info};

pub unsafe extern "system" fn wnd_proc(
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
                if let Some(state) = get_app_state() {
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

            if let Some(state) = get_app_state() {
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

            EndPaint(hwnd, &ps);
            let elapsed_us = start.elapsed().as_micros();
            log_debug!("WM_PAINT completed in {} µs.", elapsed_us);
            LRESULT(0)
        }

        WM_DPICHANGED => {
            let new_dpi = (wparam.0 & 0xFFFF) as u32;
            log_info!("WM_DPICHANGED received -> new DPI = {}", new_dpi);
            if let Some(state) = get_app_state() {
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
                    HWND::default(),
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
            if let Some(state) = get_app_state() {
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

            let cur_w = if let Some(state) = get_app_state() {
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

            // Hit-test WelcomeCard [ Got it — Dismiss ] button
            if let Some(state) = get_app_state() {
                let mut cfg = state.config.lock().unwrap();
                if cfg.first_run {
                    let scroll_y = state.scroll_offset_y.load(Ordering::Relaxed);
                    let scale = cfg.font_size.scale();
                    let card_top = 52 - scroll_y;
                    let btn_top = card_top + ((136.0 - 45.0) * scale).round() as i32;
                    let btn_bottom = btn_top + (34.0 * scale).round() as i32;
                    let btn_w = (160.0 * scale).round() as i32;
                    if x >= 14 && x <= (14 + btn_w) && y >= btn_top && y <= btn_bottom {
                        log_info!("User clicked [ Got it — Dismiss ] on WelcomeCard.");
                        cfg.first_run = false;
                        let _ = cfg.save();
                        drop(cfg);
                        let _ = InvalidateRect(hwnd, None, false);
                        return LRESULT(0);
                    }
                }
            }

            LRESULT(0)
        }

        WM_TIMER => {
            if let Some(state) = get_app_state() {
                let config = state.config.lock().unwrap().clone();

                // Check fullscreen auto-pause
                if config.auto_pause_fullscreen {
                    let is_fs = FullscreenDetector::is_foreground_fullscreen();
                    state.telemetry.set_paused(is_fs);
                }

                // Check Caffeine Mode auto-timeout
                if config.caffeine_enabled && config.caffeine_timeout_mins > 0 {
                    if let Ok(mut start_guard) = state.caffeine_start_time.try_lock() {
                        if let Some(start_time) = *start_guard {
                            let timeout_secs = config.caffeine_timeout_mins as u64 * 60;
                            if start_time.elapsed().as_secs() >= timeout_secs {
                                log_info!(
                                    "Caffeine auto-timeout elapsed ({} mins). Restoring normal power state.",
                                    config.caffeine_timeout_mins
                                );
                                windows::Win32::System::Power::SetThreadExecutionState(
                                    windows::Win32::System::Power::ES_CONTINUOUS,
                                );
                                *start_guard = None;
                                let mut cfg = state.config.lock().unwrap();
                                cfg.caffeine_enabled = false;
                                let _ = cfg.save();
                            }
                        }
                    }
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
                    if let Some(state) = get_app_state() {
                        let action = {
                            if let Ok(tray) = state.tray.lock() {
                                let config = state.config.lock().unwrap().clone();
                                tray.show_context_menu(&config)
                            } else {
                                0
                            }
                        };
                        handle_tray_menu_action(hwnd, action, state);
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
            if let Some(state) = get_app_state() {
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
