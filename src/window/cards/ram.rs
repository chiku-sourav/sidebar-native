use windows::Win32::Graphics::Gdi::{SelectObject, SetTextColor, HDC};

use crate::config::AppConfig;
use crate::telemetry::TelemetrySnapshot;
use crate::window::cards::CardRenderer;
use crate::window::context::RenderContext;

pub struct RamCard;

impl RamCard {
    pub fn new() -> Self {
        Self
    }
}

impl CardRenderer for RamCard {
    fn name(&self) -> &'static str {
        "System Memory (RAM)"
    }

    fn is_enabled(&self, config: &AppConfig) -> bool {
        config.show_ram
    }

    fn calculate_height(&self, _snapshot: &TelemetrySnapshot, config: &AppConfig) -> i32 {
        let scale = config.font_size.scale();
        (126.0 * scale).round() as i32
    }

    fn render(
        &self,
        ctx: &RenderContext,
        hdc: HDC,
        x: i32,
        y: i32,
        w: i32,
        snapshot: &TelemetrySnapshot,
        config: &AppConfig,
    ) {
        unsafe {
            let card_h = self.calculate_height(snapshot, config);
            ctx.draw_card(hdc, x, y, w, card_h, ctx.pal.bg_card, ctx.pal.card_border);

            let mut inside_y = y + ctx.lh(11);
            SelectObject(hdc, ctx.hfont_header);
            SetTextColor(hdc, ctx.pal.text_muted);
            ctx.draw_text(hdc, x + 14, inside_y, "SYSTEM MEMORY (RAM)");

            let ram_used_gb = snapshot.ram.used_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
            let ram_total_gb = snapshot.ram.total_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
            let ram_cached_gb = snapshot.ram.cached_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
            let ram_free_gb = snapshot.ram.free_bytes as f32 / (1024.0 * 1024.0 * 1024.0);

            inside_y += ctx.lh(20);
            ctx.draw_key_value(
                hdc,
                x + 14,
                inside_y,
                w - 28,
                "Memory in Use",
                &format!(
                    "{:.1} GB / {:.1} GB ({:.0}%)",
                    ram_used_gb, ram_total_gb, snapshot.ram.usage_percentage
                ),
                ctx.pal.text_muted,
                ctx.pal.accent_amber,
            );

            inside_y += ctx.lh(24);
            let ram_bar_w = w - 28;
            let ram_fill = ((snapshot.ram.usage_percentage / 100.0) * ram_bar_w as f32) as i32;
            ctx.draw_progress_bar(
                hdc,
                x + 14,
                inside_y,
                ram_bar_w,
                ctx.lh(7).max(5),
                ram_fill,
                ctx.pal.accent_amber,
                ctx.pal.progress_track,
            );

            inside_y += ctx.lh(22);
            ctx.draw_dot_row(
                hdc,
                x + 14,
                inside_y,
                &format!("In use ({:.1}G)", ram_used_gb),
                ctx.pal.accent_amber,
                &format!("Cached ({:.1}G)", ram_cached_gb),
                ctx.pal.accent_cyan,
                &format!("Free ({:.1}G)", ram_free_gb),
                ctx.pal.text_muted,
            );
        }
    }
}
