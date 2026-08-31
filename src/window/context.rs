#![allow(unused_imports, dead_code, unused_must_use)]

use crate::config::{AppConfig, AppTheme};
use windows::Win32::Foundation::{COLORREF, HWND, RECT, SIZE};
use windows::Win32::Graphics::Gdi::{
    CreatePen, CreateSolidBrush, DeleteObject, Ellipse, GetTextExtentPoint32W, LineTo, MoveToEx,
    RoundRect, SelectObject, SetBkMode, SetTextColor, TextOutW, HDC, HFONT, PS_SOLID, TRANSPARENT,
};

#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    pub bg_window: COLORREF,
    pub bg_card: COLORREF,
    pub card_border: COLORREF,
    pub text_primary: COLORREF,
    pub text_secondary: COLORREF,
    pub text_muted: COLORREF,
    pub accent_teal: COLORREF,
    pub accent_purple: COLORREF,
    pub accent_cyan: COLORREF,
    pub accent_amber: COLORREF,
    pub accent_green: COLORREF,
    pub accent_red: COLORREF,
    pub progress_track: COLORREF,
    pub scroll_thumb: COLORREF,
}

impl ThemePalette {
    pub fn resolve(theme: AppTheme, is_system_dark: bool) -> Self {
        let is_dark = match theme {
            AppTheme::Auto => is_system_dark,
            AppTheme::DarkSlate | AppTheme::OledBlack | AppTheme::Nord | AppTheme::Cyberpunk => {
                true
            }
            AppTheme::LightMode => false,
        };

        match theme {
            AppTheme::LightMode => Self {
                bg_window: COLORREF(0x00F5F2EF),
                bg_card: COLORREF(0x00FFFFFF),
                card_border: COLORREF(0x00DDD5CC),
                text_primary: COLORREF(0x001C1A18),
                text_secondary: COLORREF(0x005A534E),
                text_muted: COLORREF(0x008E847C),
                accent_teal: COLORREF(0x00806600),
                accent_purple: COLORREF(0x00883377),
                accent_cyan: COLORREF(0x00996600),
                accent_amber: COLORREF(0x000088DD),
                accent_green: COLORREF(0x002E7D32),
                accent_red: COLORREF(0x003333D3),
                progress_track: COLORREF(0x00E2DCD5),
                scroll_thumb: COLORREF(0x00C4BCB4),
            },
            AppTheme::OledBlack => Self {
                bg_window: COLORREF(0x00000000),
                bg_card: COLORREF(0x00080808),
                card_border: COLORREF(0x001E1E1E),
                text_primary: COLORREF(0x00FFFFFF),
                text_secondary: COLORREF(0x00B0B0B0),
                text_muted: COLORREF(0x00707070),
                accent_teal: COLORREF(0x00E0A526),
                accent_purple: COLORREF(0x00D970B0),
                accent_cyan: COLORREF(0x00D8C040),
                accent_amber: COLORREF(0x0038BDF8),
                accent_green: COLORREF(0x004ADE80),
                accent_red: COLORREF(0x004755F8),
                progress_track: COLORREF(0x00151515),
                scroll_thumb: COLORREF(0x00404040),
            },
            AppTheme::Nord => Self {
                bg_window: COLORREF(0x003B2E24),
                bg_card: COLORREF(0x0043342E),
                card_border: COLORREF(0x0054433B),
                text_primary: COLORREF(0x00ECEFF4),
                text_secondary: COLORREF(0x00D8DEE9),
                text_muted: COLORREF(0x00928072),
                accent_teal: COLORREF(0x00C0A388),
                accent_purple: COLORREF(0x00BA88B4),
                accent_cyan: COLORREF(0x00D0BC8F),
                accent_amber: COLORREF(0x006CBBEB),
                accent_green: COLORREF(0x007AA8A3),
                accent_red: COLORREF(0x006561BF),
                progress_track: COLORREF(0x004C3A31),
                scroll_thumb: COLORREF(0x00614D43),
            },
            AppTheme::Cyberpunk => Self {
                bg_window: COLORREF(0x00140A05),
                bg_card: COLORREF(0x001F0F0B),
                card_border: COLORREF(0x00451B12),
                text_primary: COLORREF(0x0000FFFF),
                text_secondary: COLORREF(0x00FF00EA),
                text_muted: COLORREF(0x00806050),
                accent_teal: COLORREF(0x0000F0FF),
                accent_purple: COLORREF(0x00D000FF),
                accent_cyan: COLORREF(0x00FFB800),
                accent_amber: COLORREF(0x0000D0FF),
                accent_green: COLORREF(0x0000FF70),
                accent_red: COLORREF(0x003030FF),
                progress_track: COLORREF(0x002B120B),
                scroll_thumb: COLORREF(0x00801840),
            },
            AppTheme::DarkSlate | AppTheme::Auto => {
                if !is_dark {
                    Self::resolve(AppTheme::LightMode, false)
                } else {
                    Self {
                        bg_window: COLORREF(0x001C1410),
                        bg_card: COLORREF(0x00281E18),
                        card_border: COLORREF(0x003D2E26),
                        text_primary: COLORREF(0x00F5F2F0),
                        text_secondary: COLORREF(0x00C4B8B0),
                        text_muted: COLORREF(0x008A7C74),
                        accent_teal: COLORREF(0x00DEC038),
                        accent_purple: COLORREF(0x00C770B0),
                        accent_cyan: COLORREF(0x00E0BC40),
                        accent_amber: COLORREF(0x0038BDF8),
                        accent_green: COLORREF(0x004ADE80),
                        accent_red: COLORREF(0x004B55F4),
                        progress_track: COLORREF(0x0033261F),
                        scroll_thumb: COLORREF(0x00554238),
                    }
                }
            }
        }
    }
}

pub struct RenderContext {
    pub pal: ThemePalette,
    pub font_scale: f32,
    pub hfont_title: HFONT,
    pub hfont_big_pct: HFONT,
    pub hfont_clock: HFONT,
    pub hfont_tag: HFONT,
    pub hfont_header: HFONT,
    pub hfont_label: HFONT,
    pub hfont_value: HFONT,
    pub hfont_caption: HFONT,
}

impl RenderContext {
    pub fn lh(&self, base_px: i32) -> i32 {
        ((base_px as f32) * self.font_scale).round() as i32
    }
    pub unsafe fn draw_card(
        &self,
        hdc: HDC,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        bg_col: COLORREF,
        border_col: COLORREF,
    ) {
        let brush = CreateSolidBrush(bg_col);
        let pen = CreatePen(PS_SOLID, 1, border_col);
        let old_brush = SelectObject(hdc, brush);
        let old_pen = SelectObject(hdc, pen);

        let _ = RoundRect(hdc, x, y, x + w, y + h, 14, 14);

        SelectObject(hdc, old_brush);
        SelectObject(hdc, old_pen);
        DeleteObject(brush);
        DeleteObject(pen);
    }

    pub unsafe fn draw_progress_bar(
        &self,
        hdc: HDC,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        fill_w: i32,
        fill_col: COLORREF,
        track_col: COLORREF,
    ) {
        let track_brush = CreateSolidBrush(track_col);
        let track_pen = CreatePen(PS_SOLID, 1, track_col);
        let old_b = SelectObject(hdc, track_brush);
        let old_p = SelectObject(hdc, track_pen);
        let _ = RoundRect(hdc, x, y, x + w, y + h, h, h);
        SelectObject(hdc, old_b);
        SelectObject(hdc, old_p);
        DeleteObject(track_brush);
        DeleteObject(track_pen);

        if fill_w > 2 {
            let fill_brush = CreateSolidBrush(fill_col);
            let fill_pen = CreatePen(PS_SOLID, 1, fill_col);
            let old_fb = SelectObject(hdc, fill_brush);
            let old_fp = SelectObject(hdc, fill_pen);
            let _ = RoundRect(hdc, x, y, x + fill_w.min(w), y + h, h, h);
            SelectObject(hdc, old_fb);
            SelectObject(hdc, old_fp);
            DeleteObject(fill_brush);
            DeleteObject(fill_pen);
        }
    }

    pub unsafe fn draw_multi_core_grid(
        &self,
        hdc: HDC,
        x: i32,
        y: i32,
        w: i32,
        core_usages: &[f32],
        fill_col: COLORREF,
        track_col: COLORREF,
    ) {
        let count = core_usages.len();
        if count == 0 {
            return;
        }
        let gap = 2;
        let single_w = ((w - (gap * (count as i32 - 1))) / count as i32).max(3);

        for (i, usage) in core_usages.iter().enumerate() {
            let cur_x = x + (i as i32 * (single_w + gap));
            let fill_w = ((*usage / 100.0) * single_w as f32) as i32;
            self.draw_progress_bar(hdc, cur_x, y, single_w, 4, fill_w, fill_col, track_col);
        }
    }

    pub unsafe fn draw_key_value(
        &self,
        hdc: HDC,
        x_left: i32,
        y: i32,
        width: i32,
        key: &str,
        value: &str,
        key_color: COLORREF,
        val_color: COLORREF,
    ) {
        if width <= 20 {
            return;
        }

        // 1. Measure and truncate value if it exceeds 52% of total width
        SelectObject(hdc, self.hfont_value);
        SetTextColor(hdc, val_color);

        let max_val_px = (width * 52 / 100).max(50);
        let mut val_truncated = value.to_string();
        let mut val_wstr = format!("{}\0", val_truncated)
            .encode_utf16()
            .collect::<Vec<u16>>();
        let mut val_size = SIZE::default();
        let _ = GetTextExtentPoint32W(hdc, &val_wstr[..val_wstr.len() - 1], &mut val_size);

        while val_size.cx > max_val_px && val_truncated.len() > 3 {
            val_truncated.pop();
            let test_str = format!("{}...", val_truncated.trim_end());
            let test_wstr = format!("{}\0", test_str)
                .encode_utf16()
                .collect::<Vec<u16>>();
            let _ = GetTextExtentPoint32W(hdc, &test_wstr[..test_wstr.len() - 1], &mut val_size);
            if val_size.cx <= max_val_px {
                val_wstr = test_wstr;
                break;
            }
        }

        let val_px_w = val_size.cx;
        let x_val = (x_left + width - val_px_w).max(x_left + 40);
        TextOutW(hdc, x_val, y, &val_wstr[..val_wstr.len() - 1]);

        // 2. Measure and truncate key to fit strictly within remaining space
        SelectObject(hdc, self.hfont_label);
        SetTextColor(hdc, key_color);

        let max_key_px = (x_val - x_left - 8).max(20);
        let mut key_truncated = key.to_string();
        let mut key_wstr = format!("{}\0", key_truncated)
            .encode_utf16()
            .collect::<Vec<u16>>();
        let mut key_size = SIZE::default();
        let _ = GetTextExtentPoint32W(hdc, &key_wstr[..key_wstr.len() - 1], &mut key_size);

        while key_size.cx > max_key_px && key_truncated.len() > 3 {
            key_truncated.pop();
            let test_str = format!("{}...", key_truncated.trim_end());
            let test_wstr = format!("{}\0", test_str)
                .encode_utf16()
                .collect::<Vec<u16>>();
            let _ = GetTextExtentPoint32W(hdc, &test_wstr[..test_wstr.len() - 1], &mut key_size);
            if key_size.cx <= max_key_px {
                key_wstr = test_wstr;
                break;
            }
        }

        TextOutW(hdc, x_left, y, &key_wstr[..key_wstr.len() - 1]);
    }

    pub unsafe fn draw_dot_row(
        &self,
        hdc: HDC,
        x: i32,
        y: i32,
        label1: &str,
        col1: COLORREF,
        label2: &str,
        col2: COLORREF,
        label3: &str,
        col3: COLORREF,
    ) {
        SelectObject(hdc, self.hfont_tag);
        let mut cur_x = x;

        // Dot 1
        self.draw_colored_dot(hdc, cur_x, y + 4, col1);
        cur_x += 10;
        SetTextColor(hdc, self.pal.text_muted);
        let wstr1 = format!("{}\0", label1).encode_utf16().collect::<Vec<u16>>();
        let len1 = wstr1.len() - 1;
        TextOutW(hdc, cur_x, y, &wstr1[..len1]);
        let mut s1 = SIZE::default();
        GetTextExtentPoint32W(hdc, &wstr1[..len1], &mut s1);
        cur_x += s1.cx + 12;

        // Dot 2
        self.draw_colored_dot(hdc, cur_x, y + 4, col2);
        cur_x += 10;
        let wstr2 = format!("{}\0", label2).encode_utf16().collect::<Vec<u16>>();
        let len2 = wstr2.len() - 1;
        TextOutW(hdc, cur_x, y, &wstr2[..len2]);
        let mut s2 = SIZE::default();
        GetTextExtentPoint32W(hdc, &wstr2[..len2], &mut s2);
        cur_x += s2.cx + 12;

        // Dot 3
        self.draw_colored_dot(hdc, cur_x, y + 4, col3);
        cur_x += 10;
        let wstr3 = format!("{}\0", label3).encode_utf16().collect::<Vec<u16>>();
        let len3 = wstr3.len() - 1;
        TextOutW(hdc, cur_x, y, &wstr3[..len3]);
    }

    pub unsafe fn draw_colored_dot(&self, hdc: HDC, cx: i32, cy: i32, color: COLORREF) {
        let brush = CreateSolidBrush(color);
        let pen = CreatePen(PS_SOLID, 1, color);
        let old_brush = SelectObject(hdc, brush);
        let old_pen = SelectObject(hdc, pen);

        let r = 3;
        let _ = Ellipse(hdc, cx - r, cy - r, cx + r, cy + r);

        SelectObject(hdc, old_brush);
        SelectObject(hdc, old_pen);
        DeleteObject(brush);
        DeleteObject(pen);
    }

    pub unsafe fn draw_text(&self, hdc: HDC, x: i32, y: i32, text: &str) {
        let wstr = format!("{}\0", text).encode_utf16().collect::<Vec<u16>>();
        TextOutW(hdc, x, y, &wstr[..wstr.len() - 1]);
    }
}
