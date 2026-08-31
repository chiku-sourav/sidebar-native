use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Gdi::{
    CreatePen, CreateSolidBrush, DeleteObject, Ellipse, LineTo, MoveToEx, RoundRect, SelectObject,
    TextOutW, HDC, PS_SOLID,
};

use super::RenderContext;

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
        use windows::Win32::Foundation::SIZE;
        use windows::Win32::Graphics::Gdi::{GetTextExtentPoint32W, SetTextColor};

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

    pub unsafe fn draw_text(&self, hdc: HDC, x: i32, y: i32, text: &str) {
        let wstr = format!("{}\0", text).encode_utf16().collect::<Vec<u16>>();
        TextOutW(hdc, x, y, &wstr[..wstr.len() - 1]);
    }
}

