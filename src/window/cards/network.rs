use windows::Win32::Graphics::Gdi::{SelectObject, SetTextColor, HDC};

use crate::config::AppConfig;
use crate::telemetry::process::format_speed;
use crate::telemetry::TelemetrySnapshot;
use crate::window::cards::CardRenderer;
use crate::window::context::RenderContext;

pub struct NetworkCard;

impl NetworkCard {
    pub fn new() -> Self {
        Self
    }
}

impl CardRenderer for NetworkCard {
    fn name(&self) -> &'static str {
        "Network I/O"
    }

    fn is_enabled(&self, config: &AppConfig) -> bool {
        config.show_network
    }

    fn calculate_height(&self, snapshot: &TelemetrySnapshot, config: &AppConfig) -> i32 {
        let scale = config.font_size.scale();
        let active_adapters: Vec<_> = snapshot.network.adapters.iter()
            .filter(|a| a.is_up || config.show_disabled_hardware)
            .take(3)
            .collect();

        if active_adapters.is_empty() {
            (110.0 * scale).round() as i32
        } else {
            ((52.0 + (active_adapters.len() as f32 * 46.0)) * scale).round() as i32
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

            let mut inside_y = y + ctx.lh(12);
            SelectObject(hdc, ctx.hfont_header);
            SetTextColor(hdc, ctx.pal.text_muted);
            ctx.draw_text(hdc, x + 14, inside_y, "NETWORK I/O & ACTIVE ADAPTERS");

            inside_y += ctx.lh(20);

            let adapters: Vec<_> = snapshot.network.adapters.iter()
                .filter(|a| a.is_up || config.show_disabled_hardware)
                .take(3)
                .collect();

            if adapters.is_empty() {
                SelectObject(hdc, ctx.hfont_caption);
                SetTextColor(hdc, ctx.pal.text_muted);
                ctx.draw_text(hdc, x + 14, inside_y, "No active network adapters found.");
            } else {
                for (i, adapter) in adapters.iter().enumerate() {
                    // Line 1: Adapter Name & IP
                    let dot_col = if adapter.is_up { ctx.pal.accent_green } else { ctx.pal.text_muted };
                    ctx.draw_colored_dot(hdc, x + 14, inside_y + 4, dot_col);

                    let adapter_name = if adapter.name.len() > 24 {
                        format!("{}...", &adapter.name[..22])
                    } else {
                        adapter.name.clone()
                    };

                    ctx.draw_key_value(
                        hdc,
                        x + 24,
                        inside_y,
                        w - 38,
                        &adapter_name,
                        &adapter.ip,
                        if adapter.is_up { ctx.pal.text_primary } else { ctx.pal.text_muted },
                        ctx.pal.text_muted,
                    );

                    inside_y += ctx.lh(20);

                    // Line 2: Dedicated Download & Upload Speeds
                    let dl_str = format_speed(adapter.download_bytes_sec);
                    let ul_str = format_speed(adapter.upload_bytes_sec);
                    let speed_str = format!("↓ {} • ↑ {}", dl_str, ul_str);

                    ctx.draw_key_value(
                        hdc,
                        x + 24,
                        inside_y,
                        w - 38,
                        "Throughput",
                        &speed_str,
                        ctx.pal.text_muted,
                        ctx.pal.accent_cyan,
                    );

                    inside_y += ctx.lh(23);
                    if i + 1 < adapters.len() {
                        inside_y += ctx.lh(3);
                    }
                }
            }
        }
    }
}
