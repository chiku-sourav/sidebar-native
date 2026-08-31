use windows::Win32::Graphics::Gdi::{SelectObject, SetTextColor, HDC};

use crate::config::AppConfig;
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
        let limit = config.process_limit_per_category.max(1);

        let mut total_h = 32.0; // Header + card top/bottom padding

        let mut active_sections = 0;

        if config.show_top_cpu {
            active_sections += 1;
            let count = snapshot.top_cpu_processes.len().min(limit).max(1);
            total_h += 22.0 + (count as f32 * 21.0);
        }

        if config.show_top_ram {
            active_sections += 1;
            let count = snapshot.top_ram_processes.len().min(limit).max(1);
            total_h += 22.0 + (count as f32 * 21.0);
        }

        if config.show_top_disk {
            active_sections += 1;
            let active_disk_count = snapshot
                .top_disk_processes
                .iter()
                .filter(|p| p.disk_total_bytes_sec > 0)
                .count()
                .min(limit);
            let count = if active_disk_count > 0 {
                active_disk_count
            } else {
                1
            };
            total_h += 22.0 + (count as f32 * 21.0);
        }

        if config.show_top_network {
            active_sections += 1;
            let active_net_count = snapshot
                .top_network_processes
                .iter()
                .filter(|p| {
                    p.net_total_bytes_sec > 0 || p.active_sockets > 0 || p.disk_total_bytes_sec > 0
                })
                .count()
                .min(limit);
            let count = if active_net_count > 0 {
                active_net_count
            } else {
                1
            };
            // +1 row for the "Run as Admin" hint when ETW is not active
            let hint_rows = if !snapshot.etw_network_active { 1 } else { 0 };
            total_h += 22.0 + ((count + hint_rows) as f32 * 21.0);
        }

        if active_sections > 1 {
            total_h += (active_sections - 1) as f32 * 8.0;
        }

        if active_sections == 0 {
            total_h = 64.0;
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

            let mut inside_y = y + ctx.lh(11);
            SelectObject(hdc, ctx.hfont_header);
            SetTextColor(hdc, ctx.pal.text_muted);
            ctx.draw_text(hdc, x + 14, inside_y, "TOP PROCESSES BY RESOURCE");

            inside_y += ctx.lh(20);
            let limit = config.process_limit_per_category.max(1);

            let mut rendered_any = false;

            // ==========================================
            // 1. TOP PROCESSES BY CPU
            // ==========================================
            if config.show_top_cpu {
                if rendered_any {
                    inside_y += ctx.lh(6);
                }
                rendered_any = true;

                ctx.draw_colored_dot(hdc, x + 14, inside_y + 4, ctx.pal.accent_cyan);
                SelectObject(hdc, ctx.hfont_caption);
                SetTextColor(hdc, ctx.pal.accent_cyan);
                ctx.draw_text(hdc, x + 24, inside_y, "CPU USAGE");
                inside_y += ctx.lh(18);

                let cpu_procs: Vec<_> = snapshot.top_cpu_processes.iter().take(limit).collect();
                if cpu_procs.is_empty() {
                    SelectObject(hdc, ctx.hfont_caption);
                    SetTextColor(hdc, ctx.pal.text_muted);
                    ctx.draw_text(hdc, x + 24, inside_y, "Scanning CPU processes...");
                    inside_y += ctx.lh(21);
                } else {
                    for proc in cpu_procs {
                        let key = format!("{} ({:.1}%)", proc.name, proc.cpu_usage);
                        let val_color = if proc.cpu_usage >= 10.0 {
                            ctx.pal.accent_cyan
                        } else {
                            ctx.pal.text_muted
                        };

                        ctx.draw_key_value(
                            hdc,
                            x + 24,
                            inside_y,
                            w - 38,
                            &key,
                            &proc.formatted_memory,
                            ctx.pal.text_primary,
                            val_color,
                        );
                        inside_y += ctx.lh(21);
                    }
                }
            }

            // ==========================================
            // 2. TOP PROCESSES BY RAM / MEMORY
            // ==========================================
            if config.show_top_ram {
                if rendered_any {
                    inside_y += ctx.lh(6);
                }
                rendered_any = true;

                ctx.draw_colored_dot(hdc, x + 14, inside_y + 4, ctx.pal.accent_amber);
                SelectObject(hdc, ctx.hfont_caption);
                SetTextColor(hdc, ctx.pal.accent_amber);
                ctx.draw_text(hdc, x + 24, inside_y, "SYSTEM MEMORY (RAM)");
                inside_y += ctx.lh(18);

                let ram_procs: Vec<_> = snapshot.top_ram_processes.iter().take(limit).collect();
                if ram_procs.is_empty() {
                    SelectObject(hdc, ctx.hfont_caption);
                    SetTextColor(hdc, ctx.pal.text_muted);
                    ctx.draw_text(hdc, x + 24, inside_y, "Scanning RAM processes...");
                    inside_y += ctx.lh(21);
                } else {
                    for proc in ram_procs {
                        let val_str = format!("{:.1}% CPU", proc.cpu_usage);
                        ctx.draw_key_value(
                            hdc,
                            x + 24,
                            inside_y,
                            w - 38,
                            &proc.name,
                            &format!("{} • {}", proc.formatted_memory, val_str),
                            ctx.pal.text_primary,
                            ctx.pal.accent_amber,
                        );
                        inside_y += ctx.lh(21);
                    }
                }
            }

            // ==========================================
            // 3. TOP PROCESSES BY DISK I/O
            // ==========================================
            if config.show_top_disk {
                if rendered_any {
                    inside_y += ctx.lh(6);
                }
                rendered_any = true;

                ctx.draw_colored_dot(hdc, x + 14, inside_y + 4, ctx.pal.accent_red);
                SelectObject(hdc, ctx.hfont_caption);
                SetTextColor(hdc, ctx.pal.accent_red);
                ctx.draw_text(hdc, x + 24, inside_y, "DISK I/O THROUGHPUT");
                inside_y += ctx.lh(18);

                let active_disk: Vec<_> = snapshot
                    .top_disk_processes
                    .iter()
                    .filter(|p| p.disk_total_bytes_sec > 0)
                    .take(limit)
                    .collect();

                if active_disk.is_empty() {
                    SelectObject(hdc, ctx.hfont_caption);
                    SetTextColor(hdc, ctx.pal.text_muted);
                    ctx.draw_text(hdc, x + 24, inside_y, "Idle (No active disk I/O activity)");
                    inside_y += ctx.lh(21);
                } else {
                    for proc in active_disk {
                        let total_speed = format_speed(proc.disk_total_bytes_sec);
                        let r_speed = format_speed(proc.disk_read_bytes_sec);
                        let w_speed = format_speed(proc.disk_write_bytes_sec);
                        let val_str = format!("{} (R: {} • W: {})", total_speed, r_speed, w_speed);

                        ctx.draw_key_value(
                            hdc,
                            x + 24,
                            inside_y,
                            w - 38,
                            &proc.name,
                            &val_str,
                            ctx.pal.text_primary,
                            ctx.pal.accent_red,
                        );
                        inside_y += ctx.lh(21);
                    }
                }
            }

            // ==========================================
            // 4. TOP PROCESSES BY NETWORK USAGE
            // ==========================================
            if config.show_top_network {
                if rendered_any {
                    inside_y += ctx.lh(6);
                }
                rendered_any = true;

                ctx.draw_colored_dot(hdc, x + 14, inside_y + 4, ctx.pal.accent_green);
                SelectObject(hdc, ctx.hfont_caption);
                SetTextColor(hdc, ctx.pal.accent_green);
                ctx.draw_text(hdc, x + 24, inside_y, "NETWORK USAGE & SOCKETS");
                inside_y += ctx.lh(18);

                let active_net: Vec<_> = snapshot
                    .top_network_processes
                    .iter()
                    .filter(|p| {
                        p.net_total_bytes_sec > 0
                            || p.active_sockets > 0
                            || p.disk_total_bytes_sec > 0
                    })
                    .take(limit)
                    .collect();

                if active_net.is_empty() {
                    SelectObject(hdc, ctx.hfont_caption);
                    SetTextColor(hdc, ctx.pal.text_muted);
                    ctx.draw_text(
                        hdc,
                        x + 24,
                        inside_y,
                        "Idle (No active network connections)",
                    );
                    inside_y += ctx.lh(21);
                } else {
                    for proc in &active_net {
                        let net_text = if proc.net_total_bytes_sec > 0 {
                            // ETW is active and we have real bandwidth data
                            let total_speed = format_speed(proc.net_total_bytes_sec);
                            let rx_speed = format_speed(proc.net_rx_bytes_sec);
                            let tx_speed = format_speed(proc.net_tx_bytes_sec);
                            if proc.active_sockets > 0 {
                                format!(
                                    "{} (↓ {} • ↑ {}) • {} conn",
                                    total_speed, rx_speed, tx_speed, proc.active_sockets
                                )
                            } else {
                                format!("{} (↓ {} • ↑ {})", total_speed, rx_speed, tx_speed)
                            }
                        } else if proc.active_sockets > 0 {
                            if proc.tcp_established > 0 {
                                format!(
                                    "{} sockets ({} estab / {} listen)",
                                    proc.active_sockets, proc.tcp_established, proc.tcp_listening
                                )
                            } else {
                                format!(
                                    "{} sockets ({} TCP / {} UDP)",
                                    proc.active_sockets, proc.tcp_sockets, proc.udp_sockets
                                )
                            }
                        } else {
                            "Active I/O".to_string()
                        };

                        ctx.draw_key_value(
                            hdc,
                            x + 24,
                            inside_y,
                            w - 38,
                            &proc.name,
                            &net_text,
                            ctx.pal.text_primary,
                            ctx.pal.accent_green,
                        );
                        inside_y += ctx.lh(21);
                    }
                }

                // When ETW did not start (no admin), show a subtle one-line hint
                // so the user understands why bandwidth figures are absent.
                if !snapshot.etw_network_active {
                    SelectObject(hdc, ctx.hfont_caption);
                    SetTextColor(hdc, ctx.pal.text_muted);
                    ctx.draw_text(
                        hdc,
                        x + 24,
                        inside_y,
                        "↑ Run as Admin to see bandwidth per app",
                    );
                    inside_y += ctx.lh(21);
                }
            }

            if !rendered_any {
                SelectObject(hdc, ctx.hfont_caption);
                SetTextColor(hdc, ctx.pal.text_muted);
                ctx.draw_text(hdc, x + 14, inside_y, "All process categories disabled.");
            }
        }
    }
}
