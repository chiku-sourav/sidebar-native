use windows::core::PCWSTR;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateBitmap, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreatePen,
    CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, FillRect, GetDC, ReleaseDC, RoundRect,
    SelectObject, SetBkMode, SetTextColor, DT_CENTER, DT_SINGLELINE, DT_VCENTER, FW_BOLD, PS_SOLID,
    TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, GetSystemMetrics, LoadIconW, LoadImageW, HICON, ICONINFO, IMAGE_ICON,
    LR_DEFAULTCOLOR, LR_SHARED, SM_CXSMICON, SM_CYSMICON,
};

pub fn load_app_icon() -> HICON {
    unsafe {
        let h_instance = GetModuleHandleW(None).unwrap_or_default();
        let cx = GetSystemMetrics(SM_CXSMICON);
        let cy = GetSystemMetrics(SM_CYSMICON);
        if let Ok(handle) = LoadImageW(
            HINSTANCE(h_instance.0),
            PCWSTR(1 as _),
            IMAGE_ICON,
            cx,
            cy,
            LR_SHARED | LR_DEFAULTCOLOR,
        ) {
            if !handle.is_invalid() && !handle.0.is_null() {
                return HICON(handle.0);
            }
        }
        LoadIconW(HINSTANCE(h_instance.0), PCWSTR(1 as _)).unwrap_or_default()
    }
}

pub fn create_pill_icon(ram_percentage: u8) -> HICON {
    unsafe {
        let screen_dc = GetDC(HWND::default());
        let mem_dc = CreateCompatibleDC(screen_dc);
        let mem_bmp = CreateCompatibleBitmap(screen_dc, 32, 32);
        let old_bmp = SelectObject(mem_dc, mem_bmp);

        let bg_brush = CreateSolidBrush(COLORREF(0x00000000));
        let rect = RECT {
            left: 0,
            top: 0,
            right: 32,
            bottom: 32,
        };
        FillRect(mem_dc, &rect, bg_brush);
        DeleteObject(bg_brush);

        let pill_brush = CreateSolidBrush(COLORREF(0x0032271D));
        let pill_pen = CreatePen(PS_SOLID, 1, COLORREF(0x00F8BD38));
        let old_brush = SelectObject(mem_dc, pill_brush);
        let old_pen = SelectObject(mem_dc, pill_pen);

        let _ = RoundRect(mem_dc, 1, 4, 31, 28, 12, 12);

        SelectObject(mem_dc, old_brush);
        SelectObject(mem_dc, old_pen);
        DeleteObject(pill_brush);
        DeleteObject(pill_pen);

        SetBkMode(mem_dc, TRANSPARENT);
        SetTextColor(mem_dc, COLORREF(0x00F8FAFC));

        let font_name = "Segoe UI\0".encode_utf16().collect::<Vec<u16>>();
        let hfont = CreateFontW(
            -13,
            0,
            0,
            0,
            FW_BOLD.0 as i32,
            0,
            0,
            0,
            1,
            0,
            0,
            0,
            0,
            PCWSTR::from_raw(font_name.as_ptr()),
        );
        let old_font = SelectObject(mem_dc, hfont);

        let mut text_rect = RECT {
            left: 2,
            top: 5,
            right: 30,
            bottom: 27,
        };
        let mut text = format!("{}%\0", ram_percentage)
            .encode_utf16()
            .collect::<Vec<u16>>();
        let text_len = text.len() - 1;
        DrawTextW(
            mem_dc,
            &mut text[..text_len],
            &mut text_rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );

        SelectObject(mem_dc, old_font);
        DeleteObject(hfont);

        // 1-bit monochrome mask bitmap with valid bit array
        let mask_bytes = [0u8; 128]; // 32x32 1-bit = 128 bytes
        let mask_bmp = CreateBitmap(32, 32, 1, 1, Some(mask_bytes.as_ptr() as *const _));

        let icon_info = ICONINFO {
            fIcon: true.into(),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask_bmp,
            hbmColor: mem_bmp,
        };

        let new_icon = CreateIconIndirect(&icon_info).unwrap_or_default();

        SelectObject(mem_dc, old_bmp);
        DeleteObject(mem_bmp);
        DeleteObject(mask_bmp);
        let _ = DeleteDC(mem_dc);
        let _ = ReleaseDC(HWND::default(), screen_dc);

        new_icon
    }
}

