use windows::Win32::Graphics::Gdi::{SelectObject, SetTextColor, HDC};

use crate::config::AppConfig;
use crate::telemetry::TelemetrySnapshot;
use crate::window::cards::CardRenderer;
use crate::window::context::RenderContext;

pub struct SystemCard;

impl SystemCard {
    pub fn new() -> Self {
        Self
    }
}

impl CardRenderer for SystemCard {
    fn name(&self) -> &'static str {
        "System Overview"
    }

    fn is_enabled(&self, config: &AppConfig) -> bool {
        config.show_system_overview
    }

    fn calculate_height(&self, _snapshot: &TelemetrySnapshot, config: &AppConfig) -> i32 {
        let scale = config.font_size.scale();
        let mut base_h = 100.0;
        if config.adv_bios {
            base_h += 64.0; // 3 extra advanced rows
        }
        (base_h * scale).round() as i32
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
            ctx.draw_text(hdc, x + 14, inside_y, "SYSTEM OVERVIEW");

            inside_y += ctx.lh(20);
            ctx.draw_key_value(
                hdc,
                x + 14,
                inside_y,
                w - 28,
                "System Uptime",
                &snapshot.ram.uptime_formatted,
                ctx.pal.text_muted,
                ctx.pal.text_primary,
            );

            inside_y += ctx.lh(22);
            ctx.draw_key_value(
                hdc,
                x + 14,
                inside_y,
                w - 28,
                "Active Tasks",
                &format!(
                    "{} procs • {} threads",
                    snapshot.ram.process_count, snapshot.ram.thread_count
                ),
                ctx.pal.text_muted,
                ctx.pal.text_primary,
            );

            // Advanced BIOS & Motherboard Details
            if config.adv_bios {
                if let Some(bios) = &snapshot.bios {
                    inside_y += ctx.lh(20);
                    let bios_ver_str =
                        format!("{} v{} ({})", bios.vendor, bios.version, bios.release_date);
                    ctx.draw_key_value(
                        hdc,
                        x + 14,
                        inside_y,
                        w - 28,
                        "Firmware (BIOS)",
                        &bios_ver_str,
                        ctx.pal.text_muted,
                        ctx.pal.accent_cyan,
                    );

                    inside_y += ctx.lh(20);
                    let sec_str = format!(
                        "UEFI • Secure Boot: {} • TPM: {}",
                        bios.secure_boot, bios.tpm_version
                    );
                    ctx.draw_key_value(
                        hdc,
                        x + 14,
                        inside_y,
                        w - 28,
                        "Platform Security",
                        &sec_str,
                        ctx.pal.text_muted,
                        ctx.pal.text_primary,
                    );

                    inside_y += ctx.lh(20);
                    let board_str =
                        format!("{} {}", bios.motherboard_mfg, bios.motherboard_product);
                    ctx.draw_key_value(
                        hdc,
                        x + 14,
                        inside_y,
                        w - 28,
                        "Motherboard",
                        &board_str,
                        ctx.pal.text_muted,
                        ctx.pal.text_muted,
                    );
                }
            }
        }
    }
}
