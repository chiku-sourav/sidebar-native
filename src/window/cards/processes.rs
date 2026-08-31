use windows::Win32::Graphics::Gdi::{SelectObject, SetTextColor, HDC};

use crate::config::{AppConfig, ProcessSortBy};
use crate::telemetry::process::format_speed;
use crate::telemetry::TelemetrySnapshot;
use crate::window::cards::CardRenderer;
use crate::window::context::RenderContext;

pub struct ProcessesCard;

impl ProcessesCard {
    pub fn new() -> Self {
        Self
    }
}

impl CardRenderer for ProcessesCard {
    fn name(&self) -> &'static str {
        "Top Processes"
    }

    fn is_enabled(&self, config: &AppConfig) -> bool {
        config.show_processes
    }

    fn calculate_height(&self, snapshot: &TelemetrySnapshot, config: &AppConfig) -> i32 {
        let scale = config.font_size.scale();
        if snapshot.top_processes.is_empty() {
            (64.0 * scale).round() as i32
        } else {
            let count = snapshot.top_processes.len().min(6);
            ((52.0 + (count as f32 * 23.0)) * scale).round() as i32
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
            let sort_title = match config.sort_processes_by {
                ProcessSortBy::Cpu => "TOP PROCESSES (CPU • RAM • DISK)",
                ProcessSortBy::Memory => "TOP PROCESSES (RAM • CPU • DISK)",
                ProcessSortBy::Disk => "TOP PROCESSES (DISK I/O • CPU • RAM)",
            };
            ctx.draw_text(hdc, x + 14, inside_y, sort_title);

            inside_y += ctx.lh(20);
            if snapshot.top_processes.is_empty() {
                SelectObject(hdc, ctx.hfont_caption);
                SetTextColor(hdc, ctx.pal.text_muted);
                ctx.draw_text(hdc, x + 14, inside_y, "Scanning active processes...");
            } else {
                for proc in snapshot.top_processes.iter().take(6) {
                    let key_text = format!("{} ({:.1}% CPU)", proc.name, proc.cpu_usage);
                    
                    let val_text = if proc.disk_total_bytes_sec > 1024 * 50 {
                        format!("{} • Disk {}", proc.formatted_memory, format_speed(proc.disk_total_bytes_sec))
                    } else {
                        proc.formatted_memory.clone()
                    };

                    let val_color = if proc.cpu_usage >= 15.0 {
                        ctx.pal.accent_cyan
                    } else if proc.disk_total_bytes_sec > 1024 * 1024 {
                        ctx.pal.accent_amber
                    } else {
                        ctx.pal.text_primary
                    };

                    ctx.draw_key_value(
                        hdc,
                        x + 14,
                        inside_y,
                        w - 28,
                        &key_text,
                        &val_text,
                        ctx.pal.text_primary,
                        val_color,
                    );
                    inside_y += ctx.lh(23);
                }
            }
        }
    }
}
