use windows::core::w;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};

use crate::config::BackdropEffect;

const DWMWA_USE_IMMERSIVE_DARK_MODE: DWMWINDOWATTRIBUTE = DWMWINDOWATTRIBUTE(20);
const DWMWA_SYSTEMBACKDROP_TYPE: DWMWINDOWATTRIBUTE = DWMWINDOWATTRIBUTE(38);

pub struct BackdropManager;

impl BackdropManager {
    pub fn is_system_dark_mode() -> bool {
        unsafe {
            let mut value: u32 = 0;
            let mut size = std::mem::size_of::<u32>() as u32;

            let res = RegGetValueW(
                HKEY_CURRENT_USER,
                w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
                w!("AppsUseLightTheme"),
                RRF_RT_REG_DWORD,
                None,
                Some(&mut value as *mut u32 as *mut _),
                Some(&mut size),
            );

            if res.is_ok() {
                return value == 0;
            }

            // Fallback check SystemUsesLightTheme
            let res_sys = RegGetValueW(
                HKEY_CURRENT_USER,
                w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
                w!("SystemUsesLightTheme"),
                RRF_RT_REG_DWORD,
                None,
                Some(&mut value as *mut u32 as *mut _),
                Some(&mut size),
            );

            if res_sys.is_ok() {
                value == 0
            } else {
                true // Default to dark mode
            }
        }
    }

    pub fn apply_backdrop(hwnd: HWND, effect: BackdropEffect, dark_mode: bool) {
        unsafe {
            // Apply Dark Mode
            let dark_mode_val: i32 = if dark_mode { 1 } else { 0 };
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                &dark_mode_val as *const _ as *const _,
                std::mem::size_of::<i32>() as u32,
            );

            // Apply Backdrop (1: Auto, 2: Mica, 3: Acrylic, 4: MicaAlt)
            let backdrop_type: i32 = match effect {
                BackdropEffect::None => 1,
                BackdropEffect::Mica => 2,
                BackdropEffect::Acrylic => 3,
                BackdropEffect::MicaAlt => 4,
            };

            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                &backdrop_type as *const _ as *const _,
                std::mem::size_of::<i32>() as u32,
            );
        }
    }
}
