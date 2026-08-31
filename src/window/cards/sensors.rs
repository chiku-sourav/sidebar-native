use windows::Win32::Graphics::Gdi::{SelectObject, SetTextColor, HDC};

use crate::config::AppConfig;
use crate::telemetry::TelemetrySnapshot;
use crate::window::cards::CardRenderer;
use crate::window::context::RenderContext;

pub struct SensorsCard;

impl SensorsCard {
    pub fn new() -> Self {
        Self
    }
}

impl CardRenderer for SensorsCard {
    fn name(&self) -> &'static str {
        "Hardware & Sensors Explorer"
    }

    fn is_enabled(&self, config: &AppConfig) -> bool {
        config.show_sensors_card
    }

    fn calculate_height(&self, snapshot: &TelemetrySnapshot, config: &AppConfig) -> i32 {
        let scale = config.font_size.scale();
        let sensors: Vec<_> = snapshot.all_sensors.iter()
            .filter(|s| s.is_active || config.show_disabled_hardware)
            .collect();

        if sensors.is_empty() {
            (70.0 * scale).round() as i32
        } else {
            ((62.0 + (sensors.len() as f32 * 23.0)) * scale).round() as i32
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
        unsafe {
            let card_h = self.calculate_height(snapshot, config);
            ctx.draw_card(hdc, x, y, w, card_h, ctx.pal.bg_card, ctx.pal.card_border);

            let mut inside_y = y + ctx.lh(14);
            SelectObject(hdc, ctx.hfont_header);
            SetTextColor(hdc, ctx.pal.text_muted);
            ctx.draw_text(hdc, x + 14, inside_y, "HARDWARE & SENSORS EXPLORER");

            let sensors: Vec<_> = snapshot.all_sensors.iter()
                .filter(|s| s.is_active || config.show_disabled_hardware)
                .collect();

            let active_count = snapshot.all_sensors.iter().filter(|s| s.is_active).count();
            let inactive_count = snapshot.all_sensors.iter().filter(|s| !s.is_active).count();

            inside_y += ctx.lh(20);
            SelectObject(hdc, ctx.hfont_caption);
            SetTextColor(hdc, ctx.pal.text_muted);
            let summary_text = if config.show_disabled_hardware && inactive_count > 0 {
                format!("{} Active Sensors • {} Disabled / Standby", active_count, inactive_count)
            } else {
                format!("{} Monitored Sensors Active", active_count)
            };
            ctx.draw_text(hdc, x + 14, inside_y, &summary_text);

            inside_y += ctx.lh(22);

            if sensors.is_empty() {
                SelectObject(hdc, ctx.hfont_caption);
                SetTextColor(hdc, ctx.pal.text_muted);
                ctx.draw_text(hdc, x + 14, inside_y, "Discovering hardware sensors...");
            } else {
                for sensor in sensors {
                    let dot_color = if sensor.is_active {
                        ctx.pal.accent_green
                    } else {
                        ctx.pal.text_muted
                    };
                    let val_color = if sensor.is_active {
                        ctx.pal.text_primary
                    } else {
                        ctx.pal.text_muted
                    };

                    // Draw status indicator dot
                    ctx.draw_colored_dot(hdc, x + 14, inside_y + 4, dot_color);

                    ctx.draw_key_value(
                        hdc,
                        x + 26,
                        inside_y,
                        w - 40,
                        &sensor.name,
                        &sensor.value,
                        if sensor.is_active { ctx.pal.text_primary } else { ctx.pal.text_muted },
                        val_color,
                    );

                    inside_y += ctx.lh(23);
                }
            }
        }
    }
}
