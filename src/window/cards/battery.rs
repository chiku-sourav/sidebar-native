use windows::Win32::Graphics::Gdi::{SelectObject, SetTextColor, HDC};

use crate::config::AppConfig;
use crate::telemetry::TelemetrySnapshot;
use crate::window::cards::CardRenderer;
use crate::window::context::RenderContext;

pub struct BatteryCard;

impl BatteryCard {
    pub fn new() -> Self {
        Self
    }
}

impl CardRenderer for BatteryCard {
    fn name(&self) -> &'static str {
        "Power & Battery"
    }

    fn is_enabled(&self, config: &AppConfig) -> bool {
        config.show_battery
    }

    fn calculate_height(&self, snapshot: &TelemetrySnapshot, config: &AppConfig) -> i32 {
        let scale = config.font_size.scale();
        if snapshot.battery.has_battery {
            (80.0 * scale).round() as i32
        } else {
            0
        }
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
        if !snapshot.battery.has_battery {
            return;
        }

        unsafe {
            let card_h = self.calculate_height(snapshot, config);
            ctx.draw_card(hdc, x, y, w, card_h, ctx.pal.bg_card, ctx.pal.card_border);

            let mut inside_y = y + ctx.lh(11);
            SelectObject(hdc, ctx.hfont_header);
            SetTextColor(hdc, ctx.pal.text_muted);
            ctx.draw_text(hdc, x + 14, inside_y, "POWER & BATTERY");

            inside_y += ctx.lh(20);
            ctx.draw_key_value(
                hdc,
                x + 14,
                inside_y,
                w - 28,
                if snapshot.battery.is_charging {
                    "Battery (Charging)"
                } else {
                    "Battery Level"
                },
                &format!("{:.0}%", snapshot.battery.percentage),
                ctx.pal.text_muted,
                if snapshot.battery.is_charging {
                    ctx.pal.accent_green
                } else {
                    ctx.pal.text_primary
                },
            );
        }
    }
}
