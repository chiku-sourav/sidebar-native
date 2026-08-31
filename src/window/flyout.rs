use std::sync::atomic::Ordering;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};
use windows::Win32::Graphics::Gdi::InvalidateRect;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, SetWindowPos, ShowWindow, HMENU, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE,
    SW_SHOWNOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use super::appbar::AppBarManager;
use super::backdrop::BackdropManager;
use super::state::{get_app_state, BASE_HEIGHT, BASE_WIDTH};
use crate::config::{AppConfig, BackdropEffect};
use crate::{log_error, log_info};

const DWMWA_WINDOW_CORNER_PREFERENCE: DWMWINDOWATTRIBUTE = DWMWINDOWATTRIBUTE(33);
const DWMWCP_ROUND: i32 = 2;

pub fn calculate_window_dimensions(dpi: u32, font_scale: f32) -> (i32, i32) {
    let dpi_scale = (dpi as f32 / 96.0).max(1.0);
    let win_w = (BASE_WIDTH as f32 * dpi_scale * (1.0 + (font_scale - 1.0) * 0.35)).round() as i32;
    let win_h = (BASE_HEIGHT as f32 * dpi_scale * (1.0 + (font_scale - 1.0) * 0.25)).round() as i32;
    (win_w, win_h)
}

pub unsafe fn create_flyout_window(
    h_instance: HINSTANCE,
    class_name: PCWSTR,
    config: &AppConfig,
    win_w: i32,
    win_h: i32,
) -> Result<HWND, windows::core::Error> {
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

    CreateWindowExW(
        ex_style,
        class_name,
        w!("SideVitals Flyout"),
        WS_POPUP,
        rect.left,
        rect.top,
        win_w,
        win_h,
        None,
        HMENU::default(),
        h_instance,
        None,
    )
}

pub unsafe fn apply_window_styling(hwnd: HWND, backdrop: BackdropEffect, is_dark: bool) {
    let corner_pref = DWMWCP_ROUND;
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWA_WINDOW_CORNER_PREFERENCE,
        &corner_pref as *const _ as *const _,
        std::mem::size_of::<i32>() as u32,
    );
    BackdropManager::apply_backdrop(hwnd, backdrop, is_dark);
}

pub fn toggle_visibility(hwnd: HWND) {
    if let Some(state) = get_app_state() {
        let currently_visible = state.is_visible.load(Ordering::Relaxed);
        log_info!("toggle_visibility: currently_visible={}", currently_visible);
        if currently_visible {
            hide_flyout(hwnd);
        } else {
            show_flyout(hwnd);
        }
    }
}

pub fn show_flyout(hwnd: HWND) {
    if let Some(state) = get_app_state() {
        log_info!("show_flyout: positioning window above tray and showing.");
        state.is_visible.store(true, Ordering::Relaxed);
        state.telemetry.set_paused(false);

        let cur_w = state.win_width.load(Ordering::Relaxed);
        let cur_h = state.win_height.load(Ordering::Relaxed);

        unsafe {
            let rect = AppBarManager::get_flyout_rect(cur_w, cur_h);
            let _ = SetWindowPos(
                hwnd,
                HWND::default(),
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

pub fn hide_flyout(hwnd: HWND) {
    if let Some(state) = get_app_state() {
        log_info!("hide_flyout: hiding window.");
        state.is_visible.store(false, Ordering::Relaxed);
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }
}
