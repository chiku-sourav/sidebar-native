use windows::Win32::Graphics::Gdi::{SelectObject, SetTextColor, HDC};

use crate::config::{AppConfig, TemperatureUnit};
use crate::telemetry::TelemetrySnapshot;
use crate::window::cards::CardRenderer;
use crate::window::context::{estimate_wrapped_lines, RenderContext};

pub struct GpuCard;

impl GpuCard {
    pub fn new() -> Self {
        Self
    }
}

impl CardRenderer for GpuCard {
    fn name(&self) -> &'static str {
        "Graphics (GPU)"
    }

    fn is_enabled(&self, config: &AppConfig) -> bool {
        config.show_gpu
    }

    fn calculate_height(&self, snapshot: &TelemetrySnapshot, config: &AppConfig) -> i32 {
        let scale = config.font_size.scale();
        let sidebar_w = config.sidebar_width.max(300);
        let gpus: Vec<_> = if config.show_all_gpus && !snapshot.gpu.gpus.is_empty() {
            snapshot
                .gpu
                .gpus
                .iter()
                .filter(|g| g.is_active || config.show_disabled_hardware)
                .collect()
        } else if !snapshot.gpu.gpus.is_empty() {
            snapshot
                .gpu
                .gpus
                .iter()
                .filter(|g| g.is_active)
                .take(1)
                .collect()
        } else {
            Vec::new()
        };

        if gpus.is_empty() {
            let has_shared = config.show_gpu_shared_memory;
            return if has_shared {
                (178.0 * scale).round() as i32
            } else {
                (132.0 * scale).round() as i32
            };
        }

        let mut total_h = 0;
        for (i, gpu) in gpus.iter().enumerate() {
            let name_lines = estimate_wrapped_lines(&gpu.name, sidebar_w - 28, scale);
            let extra_name_h = name_lines.saturating_sub(1) as f32 * 18.0;

            let card_h = if !gpu.is_active {
                ((95.0 + extra_name_h) * scale).round() as i32
            } else {
                let has_shared = config.show_gpu_shared_memory && gpu.shared_total_bytes > 0;
                let mut base_h = if has_shared { 178.0 } else { 132.0 };
                if config.adv_gpu {
                    base_h += 68.0; // 3 extra advanced rows
                }
                ((base_h + extra_name_h) * scale).round() as i32
            };
            total_h += card_h;
            if i + 1 < gpus.len() {
                total_h += 12;
            }
        }
        total_h
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
            let scale = config.font_size.scale();
            let sidebar_w = config.sidebar_width.max(300);
            let gpus_to_show: Vec<_> = if config.show_all_gpus && !snapshot.gpu.gpus.is_empty() {
                snapshot
                    .gpu
                    .gpus
                    .iter()
                    .filter(|g| g.is_active || config.show_disabled_hardware)
                    .cloned()
                    .collect()
            } else {
                vec![snapshot
                    .gpu
                    .gpus
                    .iter()
                    .find(|g| g.is_active)
                    .or_else(|| snapshot.gpu.gpus.first())
                    .cloned()
                    .unwrap_or_default()]
            };

            let mut cur_y = y;

            for gpu in gpus_to_show {
                let name_lines_est = estimate_wrapped_lines(&gpu.name, sidebar_w - 28, scale);
                let extra_name_h = name_lines_est.saturating_sub(1) as f32 * 18.0;

                if !gpu.is_active {
                    let gpu_card_h = ((95.0 + extra_name_h) * scale).round() as i32;
                    ctx.draw_card(
                        hdc,
                        x,
                        cur_y,
                        w,
                        gpu_card_h,
                        ctx.pal.bg_card,
                        ctx.pal.card_border,
                    );

                    let mut inside_y = cur_y + ctx.lh(11);
                    SelectObject(hdc, ctx.hfont_header);
                    SetTextColor(hdc, ctx.pal.text_muted);
                    ctx.draw_text(
                        hdc,
                        x + 14,
                        inside_y,
                        &format!("GRAPHICS ({}) • {} [STANDBY]", gpu.vendor, gpu.gpu_type),
                    );

                    inside_y += ctx.lh(20);
                    SelectObject(hdc, ctx.hfont_label);
                    SetTextColor(hdc, ctx.pal.text_muted);
                    let wrapped_lines = ctx.wrap_text(hdc, ctx.hfont_label, &gpu.name, w - 28);
                    for line in wrapped_lines {
                        ctx.draw_text(hdc, x + 14, inside_y, &line);
                        inside_y += ctx.lh(18);
                    }

                    inside_y += ctx.lh(4);
                    ctx.draw_key_value(
                        hdc,
                        x + 14,
                        inside_y,
                        w - 28,
                        "Adapter Power State",
                        "Inactive / Standby (D3Cold / Detached)",
                        ctx.pal.text_muted,
                        ctx.pal.accent_amber,
                    );

                    cur_y += gpu_card_h + 12;
                    continue;
                }

                let has_shared = config.show_gpu_shared_memory && gpu.shared_total_bytes > 0;
                let mut base_h = if has_shared { 178.0 } else { 132.0 };
                if config.adv_gpu {
                    base_h += 68.0;
                }
                let gpu_card_h = ((base_h + extra_name_h) * scale).round() as i32;
                ctx.draw_card(
                    hdc,
                    x,
                    cur_y,
                    w,
                    gpu_card_h,
                    ctx.pal.bg_card,
                    ctx.pal.card_border,
                );

                let mut inside_y = cur_y + ctx.lh(11);
                SelectObject(hdc, ctx.hfont_header);
                SetTextColor(hdc, ctx.pal.text_muted);
                ctx.draw_text(
                    hdc,
                    x + 14,
                    inside_y,
                    &format!("GRAPHICS ({}) • {}", gpu.vendor, gpu.gpu_type),
                );

                inside_y += ctx.lh(20);
                SelectObject(hdc, ctx.hfont_label);
                SetTextColor(hdc, ctx.pal.text_primary);
                let wrapped_lines = ctx.wrap_text(hdc, ctx.hfont_label, &gpu.name, w - 28);
                for line in wrapped_lines {
                    ctx.draw_text(hdc, x + 14, inside_y, &line);
                    inside_y += ctx.lh(18);
                }

                inside_y += ctx.lh(4);
                let used_gb = gpu.vram_used_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                let total_gb = gpu.vram_total_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                ctx.draw_key_value(
                    hdc,
                    x + 14,
                    inside_y,
                    w - 28,
                    "Dedicated VRAM",
                    &format!(
                        "{:.1} GB / {:.1} GB ({:.0}%)",
                        used_gb,
                        total_gb.max(0.1),
                        gpu.vram_usage_percentage
                    ),
                    ctx.pal.text_muted,
                    ctx.pal.accent_green,
                );

                inside_y += ctx.lh(22);
                let (temp_str, gpu_temp_color) = if let Some(gpu_temp_val) = gpu.temperature_c {
                    let color = if gpu_temp_val >= 80.0 {
                        ctx.pal.accent_red
                    } else if gpu_temp_val >= 65.0 {
                        ctx.pal.accent_amber
                    } else {
                        ctx.pal.accent_green
                    };

                    let s = match config.temperature_unit {
                        TemperatureUnit::Celsius => format!("{:.0} °C", gpu_temp_val),
                        TemperatureUnit::Fahrenheit => {
                            format!("{:.0} °F", (gpu_temp_val * 9.0 / 5.0) + 32.0)
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
                    "GPU Core Temperature",
                    &temp_str,
                    ctx.pal.text_muted,
                    gpu_temp_color,
                );

                inside_y += ctx.lh(24);
                let vram_bar_w = w - 28;
                let vram_fill = ((gpu.vram_usage_percentage / 100.0) * vram_bar_w as f32) as i32;
                ctx.draw_progress_bar(
                    hdc,
                    x + 14,
                    inside_y,
                    vram_bar_w,
                    ctx.lh(7).max(5),
                    vram_fill,
                    ctx.pal.accent_green,
                    ctx.pal.progress_track,
                );

                if has_shared {
                    inside_y += ctx.lh(18);
                    let shared_u_gb = gpu.shared_used_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                    let shared_t_gb = gpu.shared_total_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                    ctx.draw_key_value(
                        hdc,
                        x + 14,
                        inside_y,
                        w - 28,
                        "Shared System Memory",
                        &format!("{:.1} GB / {:.1} GB", shared_u_gb, shared_t_gb.max(0.1)),
                        ctx.pal.text_muted,
                        ctx.pal.text_primary,
                    );

                    inside_y += ctx.lh(22);
                    let shared_fill =
                        ((gpu.shared_usage_percentage / 100.0) * vram_bar_w as f32) as i32;
                    ctx.draw_progress_bar(
                        hdc,
                        x + 14,
                        inside_y,
                        vram_bar_w,
                        ctx.lh(7).max(5),
                        shared_fill,
                        ctx.pal.accent_amber,
                        ctx.pal.progress_track,
                    );
                }

                // Advanced GPU Details
                if config.adv_gpu {
                    inside_y += ctx.lh(22);
                    let engine_str = format!(
                        "3D: {:.0}% • Copy: {:.0}%",
                        gpu.gpu_usage_pct, gpu.copy_engine_pct
                    );
                    ctx.draw_key_value(
                        hdc,
                        x + 14,
                        inside_y,
                        w - 28,
                        "Engine Utilization",
                        &engine_str,
                        ctx.pal.text_muted,
                        ctx.pal.accent_cyan,
                    );

                    inside_y += ctx.lh(20);
                    let media_str = format!(
                        "Enc: {:.0}% • Dec: {:.0}%",
                        gpu.video_encode_pct, gpu.video_decode_pct
                    );
                    ctx.draw_key_value(
                        hdc,
                        x + 14,
                        inside_y,
                        w - 28,
                        "Video Encode / Decode",
                        &media_str,
                        ctx.pal.text_muted,
                        ctx.pal.text_primary,
                    );

                    inside_y += ctx.lh(20);
                    let driver_display = if gpu.driver_version.is_empty() {
                        "WDDM 3.1".to_string()
                    } else {
                        format!("v{}", gpu.driver_version)
                    };
                    let bus_str = if let (Some(gen), Some(width)) = (gpu.pcie_gen, gpu.pcie_width) {
                        format!("PCIe {}.0 x{}", gen, width)
                    } else if gpu.gpu_type.contains("Integrated") {
                        "Integrated Bus".to_string()
                    } else {
                        "PCIe Bus".to_string()
                    };
                    let driver_pcie = format!("{} • {}", driver_display, bus_str);
                    ctx.draw_key_value(
                        hdc,
                        x + 14,
                        inside_y,
                        w - 28,
                        "Driver & Bus",
                        &driver_pcie,
                        ctx.pal.text_muted,
                        ctx.pal.text_muted,
                    );
                }

                cur_y += gpu_card_h + 12;
            }
        }
    }
}
