use windows::Win32::Graphics::Gdi::{SelectObject, SetTextColor, HDC};

use crate::config::{AppConfig, TemperatureUnit};
use crate::telemetry::process::format_speed;
use crate::telemetry::TelemetrySnapshot;
use crate::window::cards::CardRenderer;
use crate::window::context::{estimate_wrapped_lines, RenderContext};

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
        if snapshot.storage.drives.is_empty() {
            return (64.0 * scale).round() as i32;
        }
        let sidebar_w = config.sidebar_width.max(300);
        let mut total_h = 36.0; // Header + initial padding
        for (i, drive) in snapshot.storage.drives.iter().enumerate() {
            let model_lines = estimate_wrapped_lines(&drive.model_name, sidebar_w - 28, scale);
            let mut drive_h = 76.0 + (model_lines as f32 * 18.0);
            if config.adv_storage {
                drive_h += 60.0; // 3 extra advanced rows per drive
            }
            total_h += drive_h;
            if i + 1 < snapshot.storage.drives.len() {
                total_h += 8.0;
            }
        }
        (total_h * scale).round() as i32
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
            ctx.draw_text(
                hdc,
                x + 14,
                inside_y,
                "STORAGE & PHYSICAL DRIVES (NVMe • SATA • LINUX)",
            );

            inside_y += ctx.lh(20);

            if snapshot.storage.drives.is_empty() {
                SelectObject(hdc, ctx.hfont_caption);
                SetTextColor(hdc, ctx.pal.text_muted);
                ctx.draw_text(hdc, x + 14, inside_y, "Scanning storage drives...");
            } else {
                for (i, drive) in snapshot.storage.drives.iter().enumerate() {
                    let free_gb = drive.free_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                    let tot_gb = drive.total_bytes as f32 / (1024.0 * 1024.0 * 1024.0);

                    // 1. Drive Title: Letter & Media Badge (e.g. C: • NVMe SSD or Disk 1 • SATA SSD (Linux / Ext4))
                    let drive_title = format!("{} • {}", drive.letter, drive.drive_type);
                    SelectObject(hdc, ctx.hfont_label);
                    SetTextColor(hdc, ctx.pal.text_primary);
                    ctx.draw_text(hdc, x + 14, inside_y, &drive_title);
                    inside_y += ctx.lh(20);

                    // 2. Hardware Model Name (Wrapped to next line if big)
                    SelectObject(hdc, ctx.hfont_caption);
                    SetTextColor(hdc, ctx.pal.text_muted);
                    let model_lines =
                        ctx.wrap_text(hdc, ctx.hfont_caption, &drive.model_name, w - 28);
                    for line in model_lines {
                        ctx.draw_text(hdc, x + 14, inside_y, &line);
                        inside_y += ctx.lh(18);
                    }

                    // 3. Capacity & Usage
                    let space_str = format!(
                        "{:.1} GB Free / {:.1} GB ({:.0}%)",
                        free_gb, tot_gb, drive.usage_percentage
                    );
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

                    // 4. Progress Bar
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

                    // 5. Read and Write Speeds
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

                    // Advanced Storage Details
                    if config.adv_storage {
                        inside_y += ctx.lh(20);
                        let iops_str = format!(
                            "R: {} IOPS • W: {} IOPS",
                            format_num(drive.iops_read),
                            format_num(drive.iops_write)
                        );
                        ctx.draw_key_value(
                            hdc,
                            x + 14,
                            inside_y,
                            w - 28,
                            "IOPS (R/W)",
                            &iops_str,
                            ctx.pal.text_muted,
                            ctx.pal.accent_cyan,
                        );

                        inside_y += ctx.lh(20);
                        let lat_str = format!(
                            "Lat: {:.1}ms R / {:.1}ms W • Q: {:.1}",
                            drive.read_latency_ms, drive.write_latency_ms, drive.queue_depth
                        );
                        ctx.draw_key_value(
                            hdc,
                            x + 14,
                            inside_y,
                            w - 28,
                            "Latency & Queue",
                            &lat_str,
                            ctx.pal.text_muted,
                            ctx.pal.text_primary,
                        );

                        inside_y += ctx.lh(20);
                        let temp_str = drive
                            .temperature_celsius
                            .map(|t| match config.temperature_unit {
                                TemperatureUnit::Celsius => format!(" • {:.0}°C", t),
                                TemperatureUnit::Fahrenheit => {
                                    format!(" • {:.0}°F", (t * 9.0 / 5.0) + 32.0)
                                }
                            })
                            .unwrap_or_default();
                        let sn_suffix = if drive.serial_number.is_empty() {
                            String::new()
                        } else {
                            format!(" • SN: {}", drive.serial_number)
                        };
                        let health_temp_str =
                            format!("{}{}{}", drive.health_status, temp_str, sn_suffix);
                        ctx.draw_key_value(
                            hdc,
                            x + 14,
                            inside_y,
                            w - 28,
                            "Health & Info",
                            &health_temp_str,
                            ctx.pal.text_muted,
                            ctx.pal.accent_green,
                        );
                    }

                    inside_y += ctx.lh(22);

                    if i + 1 < snapshot.storage.drives.len() {
                        inside_y += ctx.lh(8);
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
