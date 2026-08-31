use windows::Win32::Graphics::Gdi::{SelectObject, SetTextColor, HDC};

use crate::config::AppConfig;
use crate::telemetry::TelemetrySnapshot;
use crate::window::cards::CardRenderer;
use crate::window::context::{estimate_wrapped_lines, RenderContext};

pub struct AudioCard;

impl AudioCard {
    pub fn new() -> Self {
        Self
    }
}

impl CardRenderer for AudioCard {
    fn name(&self) -> &'static str {
        "Audio Playback"
    }

    fn is_enabled(&self, config: &AppConfig) -> bool {
        config.show_audio
    }

    fn calculate_height(&self, snapshot: &TelemetrySnapshot, config: &AppConfig) -> i32 {
        let scale = config.font_size.scale();
        let sidebar_w = config.sidebar_width.max(300);
        let dev_lines = estimate_wrapped_lines(&snapshot.audio.device_name, sidebar_w - 28, scale);
        let extra_h = dev_lines.saturating_sub(1) as f32 * 18.0;
        ((104.0 + extra_h) * scale).round() as i32
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
            ctx.draw_text(hdc, x + 14, inside_y, "AUDIO (MASTER PLAYBACK)");

            inside_y += ctx.lh(20);
            SelectObject(hdc, ctx.hfont_label);
            SetTextColor(hdc, ctx.pal.text_primary);
            let wrapped_dev =
                ctx.wrap_text(hdc, ctx.hfont_label, &snapshot.audio.device_name, w - 28);
            for line in wrapped_dev {
                ctx.draw_text(hdc, x + 14, inside_y, &line);
                inside_y += ctx.lh(18);
            }

            inside_y += ctx.lh(4);
            let vol_label = if snapshot.audio.is_muted {
                "Muted (0%)"
            } else {
                "Master Volume"
            };
            let vol_val_str = if snapshot.audio.is_muted {
                "MUTED".to_string()
            } else {
                format!("{:.0}%", snapshot.audio.volume_percent)
            };
            let vol_color = if snapshot.audio.is_muted {
                ctx.pal.accent_red
            } else {
                ctx.pal.accent_cyan
            };

            ctx.draw_key_value(
                hdc,
                x + 14,
                inside_y,
                w - 28,
                vol_label,
                &vol_val_str,
                ctx.pal.text_muted,
                vol_color,
            );

            inside_y += ctx.lh(24);
            let audio_bar_w = w - 28;
            let audio_fill = if snapshot.audio.is_muted {
                0
            } else {
                ((snapshot.audio.volume_percent / 100.0) * audio_bar_w as f32) as i32
            };
            ctx.draw_progress_bar(
                hdc,
                x + 14,
                inside_y,
                audio_bar_w,
                ctx.lh(7).max(5),
                audio_fill,
                ctx.pal.accent_cyan,
                ctx.pal.progress_track,
            );
        }
    }
}
