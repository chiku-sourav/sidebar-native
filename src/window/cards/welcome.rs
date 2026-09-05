use windows::Win32::Graphics::Gdi::{
    CreatePen, CreateSolidBrush, DeleteObject, RoundRect, SelectObject, SetTextColor, HDC, PS_SOLID,
};

use crate::config::AppConfig;
use crate::telemetry::TelemetrySnapshot;
use crate::window::cards::CardRenderer;
use crate::window::context::RenderContext;

pub struct WelcomeCard;

impl WelcomeCard {
    pub fn new() -> Self {
        Self
    }
}

impl CardRenderer for WelcomeCard {
    fn name(&self) -> &'static str {
        "Welcome & Quick Start"
    }

    fn is_enabled(&self, config: &AppConfig) -> bool {
        config.first_run
    }

    fn calculate_height(&self, _snapshot: &TelemetrySnapshot, config: &AppConfig) -> i32 {
        let scale = config.font_size.scale();
        (136.0 * scale).round() as i32
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
            ctx.draw_card(hdc, x, y, w, card_h, ctx.pal.bg_card, ctx.pal.accent_cyan);

            let mut inside_y = y + ctx.lh(11);
            SelectObject(hdc, ctx.hfont_header);
            SetTextColor(hdc, ctx.pal.accent_cyan);
            ctx.draw_text(
                hdc,
                x + 14,
                inside_y,
                &format!("WELCOME TO SIDEVITALS v{}", env!("CARGO_PKG_VERSION")),
            );

            inside_y += ctx.lh(20);
            SelectObject(hdc, ctx.hfont_caption);
            SetTextColor(hdc, ctx.pal.text_primary);
            ctx.draw_text(
                hdc,
                x + 14,
                inside_y,
                "Real-time native hardware diagnostics & power telemetry.",
            );

            inside_y += ctx.lh(17);
            SetTextColor(hdc, ctx.pal.text_muted);
            ctx.draw_text(
                hdc,
                x + 14,
                inside_y,
                "Right-click anywhere to customize cards, theme, and caffeine mode.",
            );

            inside_y += ctx.lh(22);
            // Dismiss button [ Got it — Dismiss ]
            let btn_w = (140.0 * ctx.font_scale).round() as i32;
            let btn_h = ctx.lh(24);

            let btn_brush = CreateSolidBrush(ctx.pal.accent_cyan);
            let btn_pen = CreatePen(PS_SOLID, 1, ctx.pal.accent_cyan);
            let old_brush = SelectObject(hdc, btn_brush);
            let old_pen = SelectObject(hdc, btn_pen);

            let _ = RoundRect(
                hdc,
                x + 14,
                inside_y,
                x + 14 + btn_w,
                inside_y + btn_h,
                6,
                6,
            );

            SelectObject(hdc, old_brush);
            SelectObject(hdc, old_pen);
            DeleteObject(btn_brush);
            DeleteObject(btn_pen);

            SelectObject(hdc, ctx.hfont_label);
            SetTextColor(hdc, ctx.pal.bg_window);
            ctx.draw_text(hdc, x + 24, inside_y + ctx.lh(4), "Got it — Dismiss");
        }
    }
}

