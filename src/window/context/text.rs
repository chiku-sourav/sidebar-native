use windows::Win32::Foundation::{COLORREF, SIZE};
use windows::Win32::Graphics::Gdi::{
    GetTextExtentPoint32W, SelectObject, SetTextColor, TextOutW, HDC, HFONT,
};

use super::RenderContext;

impl RenderContext {
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

        // 1. Measure full text extent of key
        SelectObject(hdc, self.hfont_label);
        SetTextColor(hdc, key_color);
        let key_wstr = format!("{}\0", key).encode_utf16().collect::<Vec<u16>>();
        let mut key_size = SIZE::default();
        let _ = GetTextExtentPoint32W(hdc, &key_wstr[..key_wstr.len() - 1], &mut key_size);

        // 2. Measure full text extent of value
        SelectObject(hdc, self.hfont_value);
        SetTextColor(hdc, val_color);
        let val_wstr = format!("{}\0", value).encode_utf16().collect::<Vec<u16>>();
        let mut val_size = SIZE::default();
        let _ = GetTextExtentPoint32W(hdc, &val_wstr[..val_wstr.len() - 1], &mut val_size);

        let gap = 8;
        let total_req = key_size.cx + val_size.cx + gap;

        // If both fit completely on the line without clipping:
        if total_req <= width {
            SelectObject(hdc, self.hfont_label);
            SetTextColor(hdc, key_color);
            TextOutW(hdc, x_left, y, &key_wstr[..key_wstr.len() - 1]);

            SelectObject(hdc, self.hfont_value);
            SetTextColor(hdc, val_color);
            let x_val = x_left + width - val_size.cx;
            TextOutW(hdc, x_val, y, &val_wstr[..val_wstr.len() - 1]);
            return;
        }

        // Allocate maximum pixel widths dynamically based on proportion
        let (_max_key_px, max_val_px) = if key_size.cx <= (width * 40 / 100) {
            let k_px = key_size.cx;
            (k_px, (width - k_px - gap).max(30))
        } else if val_size.cx <= (width * 45 / 100) {
            let v_px = val_size.cx;
            ((width - v_px - gap).max(30), v_px)
        } else {
            let k_px = (width * 42 / 100).max(30);
            (k_px, (width - k_px - gap).max(30))
        };

        // Truncate value if exceeding max_val_px
        SelectObject(hdc, self.hfont_value);
        SetTextColor(hdc, val_color);
        let mut val_truncated = value.to_string();
        let mut final_val_wstr = format!("{}\0", val_truncated)
            .encode_utf16()
            .collect::<Vec<u16>>();
        let mut final_val_size = val_size;

        while final_val_size.cx > max_val_px && val_truncated.len() > 3 {
            val_truncated.pop();
            let test_str = format!("{}...", val_truncated.trim_end());
            let test_wstr = format!("{}\0", test_str)
                .encode_utf16()
                .collect::<Vec<u16>>();
            let _ =
                GetTextExtentPoint32W(hdc, &test_wstr[..test_wstr.len() - 1], &mut final_val_size);
            if final_val_size.cx <= max_val_px {
                final_val_wstr = test_wstr;
                break;
            }
        }

        let val_px_w = final_val_size.cx;
        let x_val = (x_left + width - val_px_w).max(x_left + 40);

        // Truncate key if exceeding remaining space
        SelectObject(hdc, self.hfont_label);
        SetTextColor(hdc, key_color);
        let actual_max_key_px = (x_val - x_left - gap).max(20);
        let mut key_truncated = key.to_string();
        let mut final_key_wstr = format!("{}\0", key_truncated)
            .encode_utf16()
            .collect::<Vec<u16>>();
        let mut final_key_size = key_size;

        while final_key_size.cx > actual_max_key_px && key_truncated.len() > 3 {
            key_truncated.pop();
            let test_str = format!("{}...", key_truncated.trim_end());
            let test_wstr = format!("{}\0", test_str)
                .encode_utf16()
                .collect::<Vec<u16>>();
            let _ =
                GetTextExtentPoint32W(hdc, &test_wstr[..test_wstr.len() - 1], &mut final_key_size);
            if final_key_size.cx <= actual_max_key_px {
                final_key_wstr = test_wstr;
                break;
            }
        }

        TextOutW(hdc, x_left, y, &final_key_wstr[..final_key_wstr.len() - 1]);
        SelectObject(hdc, self.hfont_value);
        SetTextColor(hdc, val_color);
        TextOutW(hdc, x_val, y, &final_val_wstr[..final_val_wstr.len() - 1]);
    }

    pub unsafe fn wrap_text(
        &self,
        hdc: HDC,
        font: HFONT,
        text: &str,
        max_width_px: i32,
    ) -> Vec<String> {
        let trimmed = text.trim();
        if trimmed.is_empty() || max_width_px <= 20 {
            return vec![trimmed.to_string()];
        }

        SelectObject(hdc, font);

        // Check if entire text fits in one line first
        let full_wstr = format!("{}\0", trimmed)
            .encode_utf16()
            .collect::<Vec<u16>>();
        let mut full_sz = SIZE::default();
        let _ = GetTextExtentPoint32W(hdc, &full_wstr[..full_wstr.len() - 1], &mut full_sz);
        if full_sz.cx <= max_width_px {
            return vec![trimmed.to_string()];
        }

        let words: Vec<&str> = trimmed.split_whitespace().collect();
        if words.is_empty() {
            return vec![trimmed.to_string()];
        }

        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in words {
            let test_line = if current_line.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current_line, word)
            };

            let test_wstr = format!("{}\0", test_line)
                .encode_utf16()
                .collect::<Vec<u16>>();
            let mut test_sz = SIZE::default();
            let _ = GetTextExtentPoint32W(hdc, &test_wstr[..test_wstr.len() - 1], &mut test_sz);

            if test_sz.cx <= max_width_px {
                current_line = test_line;
            } else {
                if !current_line.is_empty() {
                    lines.push(current_line);
                    current_line = word.to_string();
                } else {
                    lines.push(word.to_string());
                    current_line = String::new();
                }
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        if lines.is_empty() {
            vec![trimmed.to_string()]
        } else {
            lines
        }
    }

    pub unsafe fn draw_wrapped_text(
        &self,
        hdc: HDC,
        font: HFONT,
        color: COLORREF,
        x: i32,
        start_y: i32,
        max_width_px: i32,
        line_height: i32,
        text: &str,
    ) -> i32 {
        SelectObject(hdc, font);
        SetTextColor(hdc, color);
        let lines = self.wrap_text(hdc, font, text, max_width_px);
        let mut cur_y = start_y;
        for line in &lines {
            self.draw_text(hdc, x, cur_y, line);
            cur_y += line_height;
        }
        lines.len() as i32 * line_height
    }
}

pub fn estimate_wrapped_lines(text: &str, max_width_px: i32, font_scale: f32) -> usize {
    let trimmed = text.trim();
    if trimmed.is_empty() || max_width_px <= 20 {
        return 1;
    }
    // Average character width for Segoe UI Variable Display at 1.0x font scale is ~7.5px
    let avg_char_w = (7.5 * font_scale).max(5.0);
    let max_chars = ((max_width_px as f32) / avg_char_w).floor() as usize;
    if max_chars == 0 || trimmed.len() <= max_chars {
        return 1;
    }

    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.is_empty() {
        return 1;
    }

    let mut lines = 1;
    let mut current_len = 0;

    for word in words {
        let w_len = word.chars().count();
        if current_len == 0 {
            current_len = w_len;
        } else if current_len + 1 + w_len <= max_chars {
            current_len += 1 + w_len;
        } else {
            lines += 1;
            current_len = w_len;
        }
    }

    lines
}
