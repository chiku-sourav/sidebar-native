#![allow(unused_imports, dead_code, unused_must_use)]

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, RECT, SIZE};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreatePen, CreateRectRgn,
    CreateSolidBrush, DeleteDC, DeleteObject, FillRect, LineTo, MoveToEx, RoundRect, SelectClipRgn,
    SelectObject, SetBkMode, SetTextColor, TextOutW, FW_BOLD, FW_DONTCARE, FW_MEDIUM, FW_SEMIBOLD,
    HDC, HFONT, PS_SOLID, SRCCOPY, TRANSPARENT,
};

use super::cards::{
    audio::AudioCard, battery::BatteryCard, cpu::CpuCard, gpu::GpuCard, header::HeaderCard,
    network::NetworkCard, processes::ProcessesCard, ram::RamCard, sensors::SensorsCard,
    storage::StorageCard, system::SystemCard, virtual_memory::VirtualMemoryCard, CardRenderer,
};
use super::context::{RenderContext, ThemePalette};
use crate::config::AppConfig;
use crate::telemetry::TelemetrySnapshot;

pub struct UIRenderer {
    pub hfont_title: HFONT,
    pub hfont_big_pct: HFONT,
    pub hfont_clock: HFONT,
    pub hfont_tag: HFONT,
    pub hfont_header: HFONT,
    pub hfont_label: HFONT,
    pub hfont_value: HFONT,
    pub hfont_caption: HFONT,
    pub current_dpi: u32,
    pub current_font_scale: f32,
    cards: Vec<Box<dyn CardRenderer>>,
}

unsafe impl Send for UIRenderer {}
unsafe impl Sync for UIRenderer {}

impl UIRenderer {
    pub fn new(dpi: u32, font_scale: f32) -> Self {
        let (
            hfont_title,
            hfont_big_pct,
            hfont_clock,
            hfont_tag,
            hfont_header,
            hfont_label,
            hfont_value,
            hfont_caption,
        ) = Self::create_fonts(dpi, font_scale);

        // Register cards following Open/Closed & Dependency Inversion Principles
        let cards: Vec<Box<dyn CardRenderer>> = vec![
            Box::new(HeaderCard::new()),
            Box::new(CpuCard::new()),
            Box::new(GpuCard::new()),
            Box::new(AudioCard::new()),
            Box::new(RamCard::new()),
            Box::new(StorageCard::new()),
            Box::new(NetworkCard::new()),
            Box::new(ProcessesCard::new()),
            Box::new(VirtualMemoryCard::new()),
            Box::new(BatteryCard::new()),
            Box::new(SystemCard::new()),
            Box::new(SensorsCard::new()),
        ];

        Self {
            hfont_title,
            hfont_big_pct,
            hfont_clock,
            hfont_tag,
            hfont_header,
            hfont_label,
            hfont_value,
            hfont_caption,
            current_dpi: dpi,
            current_font_scale: font_scale,
            cards,
        }
    }

    pub fn update_fonts(&mut self, new_dpi: u32, new_font_scale: f32) {
        if self.current_dpi == new_dpi && (self.current_font_scale - new_font_scale).abs() < 0.01 {
            return;
        }

        unsafe {
            DeleteObject(self.hfont_title);
            DeleteObject(self.hfont_big_pct);
            DeleteObject(self.hfont_clock);
            DeleteObject(self.hfont_tag);
            DeleteObject(self.hfont_header);
            DeleteObject(self.hfont_label);
            DeleteObject(self.hfont_value);
            DeleteObject(self.hfont_caption);
        }

        let (
            hfont_title,
            hfont_big_pct,
            hfont_clock,
            hfont_tag,
            hfont_header,
            hfont_label,
            hfont_value,
            hfont_caption,
        ) = Self::create_fonts(new_dpi, new_font_scale);

        self.hfont_title = hfont_title;
        self.hfont_big_pct = hfont_big_pct;
        self.hfont_clock = hfont_clock;
        self.hfont_tag = hfont_tag;
        self.hfont_header = hfont_header;
        self.hfont_label = hfont_label;
        self.hfont_value = hfont_value;
        self.hfont_caption = hfont_caption;
        self.current_dpi = new_dpi;
        self.current_font_scale = new_font_scale;
    }

    fn create_fonts(
        dpi: u32,
        font_scale: f32,
    ) -> (HFONT, HFONT, HFONT, HFONT, HFONT, HFONT, HFONT, HFONT) {
        let dpi_scale = (dpi as f32 / 96.0).max(1.0);
        let scale = dpi_scale * font_scale;
        let pc_name = w!("Segoe UI Variable Display");

        unsafe {
            let hfont_title = CreateFontW(
                (15.0 * scale).round() as i32,
                0,
                0,
                0,
                FW_BOLD.0 as i32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                pc_name,
            );

            let hfont_big_pct = CreateFontW(
                (44.0 * scale).round() as i32,
                0,
                0,
                0,
                FW_BOLD.0 as i32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                pc_name,
            );

            let hfont_clock = CreateFontW(
                (22.0 * scale).round() as i32,
                0,
                0,
                0,
                FW_BOLD.0 as i32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                pc_name,
            );

            let hfont_tag = CreateFontW(
                (12.5 * scale).round() as i32,
                0,
                0,
                0,
                FW_SEMIBOLD.0 as i32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                pc_name,
            );

            let hfont_header = CreateFontW(
                (13.5 * scale).round() as i32,
                0,
                0,
                0,
                FW_BOLD.0 as i32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                pc_name,
            );

            let hfont_label = CreateFontW(
                (14.0 * scale).round() as i32,
                0,
                0,
                0,
                FW_MEDIUM.0 as i32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                pc_name,
            );

            let hfont_value = CreateFontW(
                (14.5 * scale).round() as i32,
                0,
                0,
                0,
                FW_SEMIBOLD.0 as i32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                pc_name,
            );

            let hfont_caption = CreateFontW(
                (12.0 * scale).round() as i32,
                0,
                0,
                0,
                FW_DONTCARE.0 as i32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                pc_name,
            );

            (
                hfont_title,
                hfont_big_pct,
                hfont_clock,
                hfont_tag,
                hfont_header,
                hfont_label,
                hfont_value,
                hfont_caption,
            )
        }
    }

    /// Renders scrollable multi-option diagnostics flyout with active theme palette
    pub fn render(
        &self,
        hdc_dest: HDC,
        width: i32,
        height: i32,
        scroll_offset_y: i32,
        data: &TelemetrySnapshot,
        config: &AppConfig,
        is_system_dark: bool,
    ) -> i32 {
        unsafe {
            let mem_dc = CreateCompatibleDC(hdc_dest);
            let mem_bmp = CreateCompatibleBitmap(hdc_dest, width, height);
            let old_bmp = SelectObject(mem_dc, mem_bmp);

            SetBkMode(mem_dc, TRANSPARENT);

            let pal = ThemePalette::resolve(config.theme, is_system_dark);
            let ctx = RenderContext {
                pal,
                font_scale: config.font_size.scale(),
                hfont_title: self.hfont_title,
                hfont_big_pct: self.hfont_big_pct,
                hfont_clock: self.hfont_clock,
                hfont_tag: self.hfont_tag,
                hfont_header: self.hfont_header,
                hfont_label: self.hfont_label,
                hfont_value: self.hfont_value,
                hfont_caption: self.hfont_caption,
            };

            // Clear background
            let bg_brush = CreateSolidBrush(pal.bg_window);
            let rect_full = RECT {
                left: 0,
                top: 0,
                right: width,
                bottom: height,
            };
            FillRect(mem_dc, &rect_full, bg_brush);
            DeleteObject(bg_brush);

            let pad_x = 16;
            let card_w = width - (pad_x * 2);
            let mut cur_y = 52 - scroll_offset_y;

            // Restrict drawing of scrollable content below header
            let clip_rgn = CreateRectRgn(0, 40, width, height - 4);
            SelectClipRgn(mem_dc, clip_rgn);
            DeleteObject(clip_rgn);

            // ==========================================
            // RENDER CARDS PIPELINE (OCP & LSP)
            // ==========================================
            for card in &self.cards {
                if card.is_enabled(config) {
                    let card_h = card.calculate_height(data, config);
                    if card_h > 0 {
                        card.render(&ctx, mem_dc, pad_x, cur_y, card_w, data, config);
                        cur_y += card_h + 12;
                    }
                }
            }

            let total_content_height = cur_y + scroll_offset_y;

            // Reset clipping region to draw sticky header and scrollbar
            SelectClipRgn(mem_dc, None);

            // ==========================================
            // STICKY HEADER (y: 0..38)
            // ==========================================
            let header_bg_brush = CreateSolidBrush(pal.bg_window);
            let header_rect = RECT {
                left: 0,
                top: 0,
                right: width,
                bottom: 38,
            };
            FillRect(mem_dc, &header_rect, header_bg_brush);
            DeleteObject(header_bg_brush);

            // Header bottom border line
            let line_pen = CreatePen(PS_SOLID, 1, pal.card_border);
            let old_pen = SelectObject(mem_dc, line_pen);
            MoveToEx(mem_dc, 0, 38, None);
            LineTo(mem_dc, width, 38);
            SelectObject(mem_dc, old_pen);
            DeleteObject(line_pen);

            // Title "DIAGNOSTICS"
            SelectObject(mem_dc, self.hfont_title);
            SetTextColor(mem_dc, pal.text_muted);
            ctx.draw_text(mem_dc, pad_x + 2, 11, "DIAGNOSTICS");

            // Close button [✕] at top right
            SelectObject(mem_dc, self.hfont_title);
            SetTextColor(mem_dc, pal.text_muted);
            ctx.draw_text(mem_dc, width - pad_x - 16, 11, "✕");

            // ==========================================
            // MODERN SCROLLBAR THUMB (Windows 11 Style)
            // ==========================================
            let visible_h = height - 38;
            if total_content_height > visible_h {
                let track_top = 42;
                let track_h = height - 52;
                let thumb_h = ((visible_h as f32 / total_content_height as f32) * track_h as f32)
                    .max(28.0) as i32;
                let max_scroll = (total_content_height - visible_h).max(1);
                let thumb_top = track_top
                    + (((scroll_offset_y.min(max_scroll) as f32) / (max_scroll as f32))
                        * (track_h - thumb_h) as f32) as i32;

                let scroll_thumb_brush = CreateSolidBrush(pal.scroll_thumb);
                let scroll_pen = CreatePen(PS_SOLID, 1, pal.scroll_thumb);
                let old_b = SelectObject(mem_dc, scroll_thumb_brush);
                let old_p = SelectObject(mem_dc, scroll_pen);

                let _ = RoundRect(
                    mem_dc,
                    width - 9,
                    thumb_top,
                    width - 4,
                    thumb_top + thumb_h,
                    5,
                    5,
                );

                SelectObject(mem_dc, old_b);
                SelectObject(mem_dc, old_p);
                DeleteObject(scroll_thumb_brush);
                DeleteObject(scroll_pen);
            }

            // Blit to destination screen DC
            let _ = BitBlt(hdc_dest, 0, 0, width, height, mem_dc, 0, 0, SRCCOPY);

            SelectObject(mem_dc, old_bmp);
            DeleteObject(mem_bmp);
            let _ = DeleteDC(mem_dc);

            total_content_height
        }
    }
}

impl Drop for UIRenderer {
    fn drop(&mut self) {
        unsafe {
            DeleteObject(self.hfont_title);
            DeleteObject(self.hfont_big_pct);
            DeleteObject(self.hfont_clock);
            DeleteObject(self.hfont_tag);
            DeleteObject(self.hfont_header);
            DeleteObject(self.hfont_label);
            DeleteObject(self.hfont_value);
            DeleteObject(self.hfont_caption);
        }
    }
}
