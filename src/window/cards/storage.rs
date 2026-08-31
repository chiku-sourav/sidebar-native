use windows::Win32::Graphics::Gdi::{SelectObject, SetTextColor, HDC};

use crate::config::AppConfig;
use crate::telemetry::process::format_speed;
use crate::telemetry::TelemetrySnapshot;
use crate::window::cards::CardRenderer;
use crate::window::context::RenderContext;

pub struct StorageCard;

impl StorageCard {
    pub fn new() -> Self {
        Self
    }
}

impl CardRenderer for StorageCard {
    fn name(&self) -> &'static str {
        "Storage & Drives"
    }

    fn is_enabled(&self, config: &AppConfig) -> bool {
        config.show_storage
    }

    fn calculate_height(&self, snapshot: &TelemetrySnapshot, config: &AppConfig) -> i32 {
        let scale = config.font_size.scale();
        let drive_count = snapshot.storage.drives.len().max(1);
        // Each drive block takes header + space + bar + speeds
        ((44.0 + (drive_count as f32 * 68.0)) * scale).round() as i32
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
            ctx.draw_text(hdc, x + 14, inside_y, "STORAGE & PHYSICAL DRIVES (NVMe • SATA • LINUX)");

            inside_y += ctx.lh(20);

            if snapshot.storage.drives.is_empty() {
                SelectObject(hdc, ctx.hfont_caption);
                SetTextColor(hdc, ctx.pal.text_muted);
                ctx.draw_text(hdc, x + 14, inside_y, "Scanning storage drives...");
            } else {
                for (i, drive) in snapshot.storage.drives.iter().enumerate() {
                    let free_gb = drive.free_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                    let tot_gb = drive.total_bytes as f32 / (1024.0 * 1024.0 * 1024.0);

                    // Drive Title: Letter & Media Badge (e.g. C: [NVMe SSD] TS1TMTE400S)
                    let drive_title = format!("{} • [{}]", drive.letter, drive.drive_type);
                    let model_short = if drive.model_name.len() > 24 {
                        format!("{}...", &drive.model_name[..22])
                    } else {
                        drive.model_name.clone()
                    };

                    ctx.draw_key_value(
                        hdc,
                        x + 14,
                        inside_y,
                        w - 28,
                        &drive_title,
                        &model_short,
                        ctx.pal.text_primary,
                        ctx.pal.text_muted,
                    );

                    inside_y += ctx.lh(20);

                    // Capacity & Usage
                    let space_str = format!("{:.1} GB Free / {:.1} GB ({:.0}%)", free_gb, tot_gb, drive.usage_percentage);
                    ctx.draw_key_value(
                        hdc,
                        x + 14,
                        inside_y,
                        w - 28,
                        "Capacity",
                        &space_str,
                        ctx.pal.text_muted,
                        ctx.pal.text_primary,
                    );

                    inside_y += ctx.lh(20);

                    // Progress Bar
                    let bar_w = w - 28;
                    let fill_w = ((drive.usage_percentage / 100.0) * bar_w as f32) as i32;
                    let bar_col = if drive.usage_percentage >= 90.0 {
                        ctx.pal.accent_red
                    } else if drive.usage_percentage >= 75.0 {
                        ctx.pal.accent_amber
                    } else {
                        ctx.pal.accent_green
                    };
                    ctx.draw_progress_bar(
                        hdc,
                        x + 14,
                        inside_y,
                        bar_w,
                        ctx.lh(6).max(4),
                        fill_w,
                        bar_col,
                        ctx.pal.progress_track,
                    );

                    inside_y += ctx.lh(14);

                    // Individual Read and Write Speeds
                    let r_str = format_speed(drive.read_bytes_sec);
                    let w_str = format_speed(drive.write_bytes_sec);
                    let speed_line = format!("Read: {} • Write: {}", r_str, w_str);

                    ctx.draw_key_value(
                        hdc,
                        x + 14,
                        inside_y,
                        w - 28,
                        "Throughput",
                        &speed_line,
                        ctx.pal.text_muted,
                        ctx.pal.accent_cyan,
                    );

                    inside_y += ctx.lh(22);

                    if i + 1 < snapshot.storage.drives.len() {
                        inside_y += ctx.lh(6);
                    }
                }
            }
        }
    }
}
