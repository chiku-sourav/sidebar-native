#![allow(unused_imports, dead_code, unused_must_use)]

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_SZ,
};

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const APP_NAME: PCWSTR = w!("SidebarDiagnosticsNative");

pub struct StartupManager;

impl StartupManager {
    pub fn is_run_at_startup() -> bool {
        unsafe {
            let mut hkey = HKEY::default();
            if RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY, 0, KEY_READ, &mut hkey).is_ok() {
                let mut data_len = 0u32;
                let res = RegQueryValueExW(hkey, APP_NAME, None, None, None, Some(&mut data_len));
                let _ = RegCloseKey(hkey);
                return res.is_ok() && data_len > 0;
            }
            false
        }
    }

    pub fn set_run_at_startup(enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            let mut hkey = HKEY::default();
            if RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY, 0, KEY_WRITE, &mut hkey).is_ok() {
                if enabled {
                    if let Ok(exe_path) = std::env::current_exe() {
                        let path_str = format!("\"{}\"\0", exe_path.to_string_lossy());
                        let wide: Vec<u16> = path_str.encode_utf16().collect();
                        let bytes: &[u8] =
                            std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2);
                        let _ = RegSetValueExW(hkey, APP_NAME, 0, REG_SZ, Some(bytes));
                    }
                } else {
                    let _ = RegDeleteValueW(hkey, APP_NAME);
                }
                let _ = RegCloseKey(hkey);
            }
            Ok(())
        }
    }
}
