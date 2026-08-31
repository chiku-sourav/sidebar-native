#![allow(unused_imports, dead_code, unused_must_use)]

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, HMONITOR, MONITORINFO, MONITOR_DEFAULTTOPRIMARY};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetDesktopWindow, GetForegroundWindow, GetShellWindow, GetWindowRect, IsWindowVisible,
};

pub struct FullscreenDetector;

impl FullscreenDetector {
    pub fn is_foreground_fullscreen() -> bool {
        unsafe {
            let fg = GetForegroundWindow();
            if fg.0.is_null() {
                return false;
            }

            let desktop = GetDesktopWindow();
            let shell = GetShellWindow();
            if fg == desktop || fg == shell {
                return false;
            }

            let mut fg_rect = RECT::default();
            if GetWindowRect(fg, &mut fg_rect).is_err() {
                return false;
            }

            let h_monitor = MonitorFromWindow(fg, MONITOR_DEFAULTTOPRIMARY);
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };

            if GetMonitorInfoW(h_monitor, &mut mi).as_bool() {
                let mon_rect = mi.rcMonitor;
                // Check if foreground window covers full monitor bounds
                if fg_rect.left <= mon_rect.left
                    && fg_rect.top <= mon_rect.top
                    && fg_rect.right >= mon_rect.right
                    && fg_rect.bottom >= mon_rect.bottom
                {
                    return true;
                }
            }

            false
        }
    }
}
