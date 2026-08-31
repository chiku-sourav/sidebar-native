pub mod icon;
pub mod ids;
pub mod menu_builder;

pub use icon::{create_pill_icon, load_app_icon};
pub use ids::*;
pub use menu_builder::show_tray_popup_menu;

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::HICON;

use crate::config::AppConfig;

pub struct SystemTray {
    hwnd: HWND,
    nid: NOTIFYICONDATAW,
    current_icon: HICON,
    last_ram_pct: Option<u8>,
}

unsafe impl Send for SystemTray {}
unsafe impl Sync for SystemTray {}

impl SystemTray {
    pub fn new(hwnd: HWND) -> Self {
        let initial_icon = load_app_icon();

        let mut nid = NOTIFYICONDATAW {
            cbSize: 508, // Standard Windows V2/V3 compatible size
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: initial_icon,
            ..Default::default()
        };

        let tip = "Sidebar Diagnostics (Click to toggle flyout)\0"
            .encode_utf16()
            .collect::<Vec<u16>>();
        let len = tip.len().min(nid.szTip.len());
        nid.szTip[..len].copy_from_slice(&tip[..len]);

        Self {
            hwnd,
            nid,
            current_icon: initial_icon,
            last_ram_pct: None,
        }
    }

    pub fn register(&self) {
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &self.nid);
            let res = Shell_NotifyIconW(NIM_ADD, &self.nid);
            crate::log_info!(
                "System tray icon registered with result: {:?}",
                res.as_bool()
            );
        }
    }

    pub fn update_pill_badge(&mut self, ram_percentage: u8) {
        if self.last_ram_pct == Some(ram_percentage) {
            return;
        }

        self.last_ram_pct = Some(ram_percentage);

        let tip_str = format!(
            "Sidebar Diagnostics (RAM: {}%) - Click to toggle flyout\0",
            ram_percentage
        );
        let tip = tip_str.encode_utf16().collect::<Vec<u16>>();
        let len = tip.len().min(self.nid.szTip.len());
        self.nid.szTip = [0; 128];
        self.nid.szTip[..len].copy_from_slice(&tip[..len]);
        self.nid.uFlags = NIF_TIP;
        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &self.nid);
        }
        self.nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    }

    pub fn show_context_menu(&self, config: &AppConfig) -> u32 {
        unsafe { show_tray_popup_menu(self.hwnd, config) }
    }
}

impl Drop for SystemTray {
    fn drop(&mut self) {
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &self.nid);
        }
    }
}
