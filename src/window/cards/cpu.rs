use windows::Win32::Graphics::Gdi::{SelectObject, SetTextColor, HDC};

use crate::config::{AppConfig, TemperatureUnit};
use crate::telemetry::TelemetrySnapshot;
use crate::window::cards::CardRenderer;
use crate::window::context::{estimate_wrapped_lines, RenderContext};

pub struct CpuCard;

impl CpuCard {
    pub fn new() -> Self {
        Self
    }
}

impl CardRenderer for CpuCard {
    fn name(&self) -> &'static str {
        "Processor (CPU)"
    }

    fn is_enabled(&self, config: &AppConfig) -> bool {
        config.show_cpu
    }

    fn calculate_height(&self, snapshot: &TelemetrySnapshot, config: &AppConfig) -> i32 {
        let scale = config.font_size.scale();
        let sidebar_w = config.sidebar_width.max(300);
        let brand_lines = estimate_wrapped_lines(&snapshot.cpu.brand, sidebar_w - 28, scale);
        let extra_h = brand_lines.saturating_sub(1) as f32 * 18.0;

        let has_core_loads = config.show_core_loads && snapshot.cpu.core_usages.len() > 1;
        let mut base_h = if has_core_loads { 156.0 } else { 132.0 };
        if config.adv_cpu {
            base_h += 88.0; // 4 extra advanced detail rows
        }
        ((base_h + extra_h) * scale).round() as i32
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
            let has_core_loads = config.show_core_loads && snapshot.cpu.core_usages.len() > 1;
            let card_h = self.calculate_height(snapshot, config);
            ctx.draw_card(hdc, x, y, w, card_h, ctx.pal.bg_card, ctx.pal.card_border);

            let mut inside_y = y + ctx.lh(11);
            SelectObject(hdc, ctx.hfont_header);
            SetTextColor(hdc, ctx.pal.text_muted);
            ctx.draw_text(hdc, x + 14, inside_y, "PROCESSOR (CPU)");

            inside_y += ctx.lh(20);
            SelectObject(hdc, ctx.hfont_label);
            SetTextColor(hdc, ctx.pal.text_primary);
            let wrapped_brand = ctx.wrap_text(hdc, ctx.hfont_label, &snapshot.cpu.brand, w - 28);
            for line in wrapped_brand {
                ctx.draw_text(hdc, x + 14, inside_y, &line);
                inside_y += ctx.lh(18);
            }

            inside_y += ctx.lh(4);
            let freq_str = if snapshot.cpu.frequency_mhz > 0 {
                if config.use_ghz {
                    format!(
                        "{} Cores • {:.2} GHz",
                        snapshot.cpu.core_count,
                        snapshot.cpu.frequency_mhz as f64 / 1000.0
                    )
                } else {
                    format!(
                        "{} Cores • {} MHz",
                        snapshot.cpu.core_count, snapshot.cpu.frequency_mhz
                    )
                }
            } else {
                format!("{} Logical Cores", snapshot.cpu.core_count)
            };
            ctx.draw_key_value(
                hdc,
                x + 14,
                inside_y,
                w - 28,
                &freq_str,
                &format!("{:.1}%", snapshot.cpu.global_usage),
                ctx.pal.text_muted,
                ctx.pal.accent_cyan,
            );

            inside_y += ctx.lh(22);
            let (temp_str, temp_color) =
                if let Some(cpu_temp_val) = snapshot.temperature.cpu_package_temp {
                    let color = if cpu_temp_val >= 80.0 {
                        ctx.pal.accent_red
                    } else if cpu_temp_val >= 65.0 {
                        ctx.pal.accent_amber
                    } else {
                        ctx.pal.accent_green
                    };

                    let s = match config.temperature_unit {
                        TemperatureUnit::Celsius => format!("{:.0} °C", cpu_temp_val),
                        TemperatureUnit::Fahrenheit => {
                            format!("{:.0} °F", (cpu_temp_val * 9.0 / 5.0) + 32.0)
                        }
                    };
                    (s, color)
                } else {
                    ("N/A".to_string(), ctx.pal.text_muted)
                };

            ctx.draw_key_value(
                hdc,
                x + 14,
                inside_y,
                w - 28,
                "CPU Package Temperature",
                &temp_str,
                ctx.pal.text_muted,
                temp_color,
            );

            inside_y += ctx.lh(24);
            let cpu_bar_w = w - 28;
            let cpu_fill = ((snapshot.cpu.global_usage / 100.0) * cpu_bar_w as f32) as i32;
            ctx.draw_progress_bar(
                hdc,
                x + 14,
                inside_y,
                cpu_bar_w,
                ctx.lh(7).max(5),
                cpu_fill,
                ctx.pal.accent_cyan,
                ctx.pal.progress_track,
            );

            if has_core_loads {
                inside_y += ctx.lh(14);
                ctx.draw_multi_core_grid(
                    hdc,
                    x + 14,
                    inside_y,
                    cpu_bar_w,
                    &snapshot.cpu.core_usages,
                    ctx.pal.accent_cyan,
                    ctx.pal.progress_track,
                );
            }

            // Advanced CPU Details
            if config.adv_cpu {
                inside_y += ctx.lh(22);
                let socket_suffix = if snapshot.cpu.socket_count > 1 {
                    "s"
                } else {
                    ""
                };
                let topology_str = format!(
                    "{} Physical • {} Logical • {} Socket{}",
                    snapshot.cpu.physical_core_count,
                    snapshot.cpu.core_count,
                    snapshot.cpu.socket_count,
                    socket_suffix
                );
                ctx.draw_key_value(
                    hdc,
                    x + 14,
                    inside_y,
                    w - 28,
                    "Topology",
                    &topology_str,
                    ctx.pal.text_muted,
                    ctx.pal.text_primary,
                );

                inside_y += ctx.lh(20);
                let (base_str, boost_str) = if config.use_ghz {
                    (
                        format!("{:.2} GHz", snapshot.cpu.base_clock_mhz as f64 / 1000.0),
                        format!("{:.2} GHz", snapshot.cpu.boost_clock_mhz as f64 / 1000.0),
                    )
                } else {
                    (
                        format!("{} MHz", snapshot.cpu.base_clock_mhz),
                        format!("{} MHz", snapshot.cpu.boost_clock_mhz),
                    )
                };
                let clocks_str = format!("Base: {} • Boost: {}", base_str, boost_str);
                ctx.draw_key_value(
                    hdc,
                    x + 14,
                    inside_y,
                    w - 28,
                    "Base / Boost Clock",
                    &clocks_str,
                    ctx.pal.text_muted,
                    ctx.pal.text_primary,
                );

                inside_y += ctx.lh(20);
                let user_kernel_str = format!(
                    "User: {:.0}% • Kernel: {:.0}%",
                    snapshot.cpu.user_pct, snapshot.cpu.privileged_pct
                );
                ctx.draw_key_value(
                    hdc,
                    x + 14,
                    inside_y,
                    w - 28,
                    "User / Kernel Load",
                    &user_kernel_str,
                    ctx.pal.text_muted,
                    ctx.pal.accent_cyan,
                );

                inside_y += ctx.lh(20);
                let ctx_irq_str = format!(
                    "Ctx/s: {} • IRQ/s: {}",
                    format_num(snapshot.cpu.context_switches_per_sec),
                    format_num(snapshot.cpu.interrupts_per_sec)
                );
                ctx.draw_key_value(
                    hdc,
                    x + 14,
                    inside_y,
                    w - 28,
                    "Context / Interrupts",
                    &ctx_irq_str,
                    ctx.pal.text_muted,
                    ctx.pal.text_muted,
                );
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
