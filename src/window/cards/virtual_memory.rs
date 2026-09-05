use windows::Win32::Graphics::Gdi::{SelectObject, SetTextColor, HDC};

use crate::config::AppConfig;
use crate::telemetry::TelemetrySnapshot;
use crate::window::cards::CardRenderer;
use crate::window::context::RenderContext;

pub struct VirtualMemoryCard;

impl VirtualMemoryCard {
    pub fn new() -> Self {
        Self
    }
}

impl CardRenderer for VirtualMemoryCard {
    fn name(&self) -> &'static str {
        "Virtual Memory"
    }

    fn is_enabled(&self, config: &AppConfig) -> bool {
        config.show_virtual_memory
    }

    fn calculate_height(&self, _snapshot: &TelemetrySnapshot, config: &AppConfig) -> i32 {
        let scale = config.font_size.scale();
        let mut base_h = 138.0;
        if config.adv_virtual_memory {
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
            ctx.draw_text(hdc, x + 14, inside_y, "COMMITMENT & VIRTUAL MEMORY");

            inside_y += ctx.lh(20);
            ctx.draw_key_value(
                hdc,
                x + 14,
                inside_y,
                w - 28,
                "Committed Memory",
                &format!(
                    "{:.1} GB / {:.1} GB",
                    snapshot.ram.committed_bytes as f32 / (1024.0 * 1024.0 * 1024.0),
                    snapshot.ram.commit_limit_bytes as f32 / (1024.0 * 1024.0 * 1024.0)
                ),
                ctx.pal.text_muted,
                ctx.pal.text_primary,
            );

            inside_y += ctx.lh(22);
            ctx.draw_key_value(
                hdc,
                x + 14,
                inside_y,
                w - 28,
                "Page Faults Rate",
                &format!("{:.0} / s", snapshot.ram.page_faults_per_sec),
                ctx.pal.text_muted,
                ctx.pal.text_primary,
            );

            inside_y += ctx.lh(22);
            ctx.draw_key_value(
                hdc,
                x + 14,
                inside_y,
                w - 28,
                "Page File Used",
                &format!("{:.1}%", snapshot.ram.page_file_usage_pct),
                ctx.pal.text_muted,
                ctx.pal.text_primary,
            );

            // Advanced Virtual Memory Details
            if config.adv_virtual_memory {
                inside_y += ctx.lh(20);
                let p_gb = snapshot.ram.paged_pool_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                let np_mb = snapshot.ram.nonpaged_pool_bytes as f32 / (1024.0 * 1024.0);
                let pools_str = format!("Paged: {:.1} GB • Non-Paged: {:.0} MB", p_gb, np_mb);
                ctx.draw_key_value(
                    hdc,
                    x + 14,
                    inside_y,
                    w - 28,
                    "Kernel Pools",
                    &pools_str,
                    ctx.pal.text_muted,
                    ctx.pal.accent_cyan,
                );

                inside_y += ctx.lh(20);
                let stb_gb = snapshot.ram.standby_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                let mod_mb = snapshot.ram.modified_bytes as f32 / (1024.0 * 1024.0);
                let pages_str = format!("Standby: {:.1} GB • Modified: {:.0} MB", stb_gb, mod_mb);
                ctx.draw_key_value(
                    hdc,
                    x + 14,
                    inside_y,
                    w - 28,
                    "Page Lists",
                    &pages_str,
                    ctx.pal.text_muted,
                    ctx.pal.accent_amber,
                );

                inside_y += ctx.lh(20);
                let cache_gb = snapshot.ram.system_cache_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                ctx.draw_key_value(
                    hdc,
                    x + 14,
                    inside_y,
                    w - 28,
                    "System Cache",
                    &format!("{:.1} GB", cache_gb),
                    ctx.pal.text_muted,
                    ctx.pal.text_muted,
                );
            } else {
                inside_y += ctx.lh(24);
                SelectObject(hdc, ctx.hfont_caption);
                SetTextColor(hdc, ctx.pal.text_muted);
                ctx.draw_text(
                    hdc,
                    x + 14,
                    inside_y,
                    "Committed is what Windows has promised programs",
                );
                ctx.draw_text(
                    hdc,
                    x + 14,
                    inside_y + ctx.lh(16),
                    "— excess memory is automatically paged to storage.",
                );
            }
        }
    }
}
