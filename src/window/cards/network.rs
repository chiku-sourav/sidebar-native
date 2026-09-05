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
        let active_adapters: Vec<_> = snapshot
            .network
            .adapters
            .iter()
            .filter(|a| a.is_up || config.show_disabled_hardware)
            .take(3)
            .collect();

        if active_adapters.is_empty() {
            (110.0 * scale).round() as i32
        } else {
            let per_adapter = if config.adv_network { 86.0 } else { 46.0 };
            ((52.0 + (active_adapters.len() as f32 * per_adapter)) * scale).round() as i32
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

            let adapters: Vec<_> = snapshot
                .network
                .adapters
                .iter()
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
                    let dot_col = if adapter.is_up {
                        ctx.pal.accent_green
                    } else {
                        ctx.pal.text_muted
                    };
                    ctx.draw_colored_dot(hdc, x + 14, inside_y + 4, dot_col);

                    let title_name = if !adapter.display_name.is_empty() {
                        &adapter.display_name
                    } else {
                        &adapter.name
                    };

                    ctx.draw_key_value(
                        hdc,
                        x + 24,
                        inside_y,
                        w - 38,
                        title_name,
                        &adapter.ip,
                        if adapter.is_up {
                            ctx.pal.text_primary
                        } else {
                            ctx.pal.text_muted
                        },
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

                    // Advanced Network Details
                    if config.adv_network {
                        inside_y += ctx.lh(20);
                        let speed_gbps = if adapter.link_speed_bps >= 1_000_000_000 {
                            format!(
                                "{:.1} Gbps",
                                adapter.link_speed_bps as f64 / 1_000_000_000.0
                            )
                        } else if adapter.link_speed_bps >= 1_000_000 {
                            format!("{:.0} Mbps", adapter.link_speed_bps as f64 / 1_000_000.0)
                        } else {
                            "1.0 Gbps".to_string()
                        };

                        let mac_display = if adapter.mac_address.is_empty() {
                            "Virtual".to_string()
                        } else {
                            adapter.mac_address.clone()
                        };

                        let link_type_str = format!(
                            "{} • {} • {}",
                            adapter.adapter_type, speed_gbps, mac_display
                        );
                        ctx.draw_key_value(
                            hdc,
                            x + 24,
                            inside_y,
                            w - 38,
                            "Link & Physical",
                            &link_type_str,
                            ctx.pal.text_muted,
                            ctx.pal.text_primary,
                        );

                        inside_y += ctx.lh(20);
                        let sig_str = adapter
                            .signal_strength_pct
                            .map(|s| format!(" • Signal: {}%", s))
                            .unwrap_or_default();
                        let pkts_str = format!(
                            "Pkts: {}↓ {}↑{}",
                            format_num(adapter.packets_recv_per_sec),
                            format_num(adapter.packets_sent_per_sec),
                            sig_str
                        );
                        ctx.draw_key_value(
                            hdc,
                            x + 24,
                            inside_y,
                            w - 38,
                            "Packets & Quality",
                            &pkts_str,
                            ctx.pal.text_muted,
                            ctx.pal.text_muted,
                        );
                    }

                    inside_y += ctx.lh(23);
                    if i + 1 < adapters.len() {
                        inside_y += ctx.lh(3);
                    }
                }
            }
        }
    }
}

fn format_num(val: u64) -> String {
    if val >= 1_000_000 {
        format!("{:.1}M", val as f64 / 1_000_000.0)
    } else if val >= 10_000 {
        format!("{:.1}k", val as f64 / 1000.0)
    } else if val >= 1000 {
        let s = val.to_string();
        let len = s.len();
        format!("{},{}", &s[..len - 3], &s[len - 3..])
    } else {
        val.to_string()
    }
}
