use windows::Win32::Foundation::RECT;
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SystemParametersInfoW, SM_CXSCREEN, SM_CYSCREEN, SPI_GETWORKAREA,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
};

pub struct AppBarManager;

impl AppBarManager {
    pub fn get_flyout_rect(width: i32, height: i32) -> RECT {
        unsafe {
            let mut work_area = RECT::default();
            let success = SystemParametersInfoW(
                SPI_GETWORKAREA,
                0,
                Some(&mut work_area as *mut _ as *mut _),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            );

            let margin = 12;

            if success.is_ok() && work_area.right > work_area.left {
                let right = work_area.right - margin;
                let bottom = work_area.bottom - margin;
                let left = right - width;
                let top = bottom - height;

                RECT {
                    left,
                    top,
                    right,
                    bottom,
                }
            } else {
                let screen_w = GetSystemMetrics(SM_CXSCREEN);
                let screen_h = GetSystemMetrics(SM_CYSCREEN);
                let right = screen_w - margin;
                let bottom = screen_h - 48 - margin;
                let left = right - width;
                let top = bottom - height;

                RECT {
                    left,
                    top,
                    right,
                    bottom,
                }
            }
        }
    }
}
