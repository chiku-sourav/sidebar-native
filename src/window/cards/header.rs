use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{GetTextExtentPoint32W, SelectObject, SetTextColor, TextOutW, HDC};
use windows::Win32::System::SystemInformation::GetLocalTime;

use crate::config::{AppConfig, DateFormat};
use crate::telemetry::TelemetrySnapshot;
use crate::window::cards::CardRenderer;
use crate::window::context::RenderContext;

const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const DAYS_LONG: [&str; 7] = [
    "Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday",
];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const MONTHS_LONG: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September",
    "October", "November", "December",
];

pub struct HeaderCard;

impl HeaderCard {
    pub fn new() -> Self {
        Self
    }
}

impl CardRenderer for HeaderCard {
    fn name(&self) -> &'static str {
        "Header / Clock"
    }

    fn is_enabled(&self, config: &AppConfig) -> bool {
        config.show_machine_name || config.show_clock || config.date_format != DateFormat::Disabled
    }

    fn calculate_height(&self, _snapshot: &TelemetrySnapshot, config: &AppConfig) -> i32 {
        let scale = config.font_size.scale();
        let mut h = (18.0 * scale).round() as i32;
        if config.show_machine_name {
            h += (22.0 * scale).round() as i32;
        }
        if config.show_clock {
            h += (28.0 * scale).round() as i32;
        }
        if config.date_format != DateFormat::Disabled {
            h += (22.0 * scale).round() as i32;
        }
        h
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

            let mut inside_y = y + ctx.lh(10);
            let st = GetLocalTime();

            if config.show_machine_name {
                SelectObject(hdc, ctx.hfont_header);
                SetTextColor(hdc, ctx.pal.text_muted);
                ctx.draw_text(hdc, x + 14, inside_y, &snapshot.machine_name.to_uppercase());

                SelectObject(hdc, ctx.hfont_caption);
                SetTextColor(hdc, ctx.pal.text_muted);
                let os_wstr = format!("{}\0", snapshot.os_version)
                    .encode_utf16()
                    .collect::<Vec<u16>>();
                let mut os_sz = SIZE::default();
                GetTextExtentPoint32W(hdc, &os_wstr[..os_wstr.len() - 1], &mut os_sz);
                TextOutW(
                    hdc,
                    x + w - 14 - os_sz.cx,
                    inside_y,
                    &os_wstr[..os_wstr.len() - 1],
                );

                inside_y += ctx.lh(22);
            }

            if config.show_clock {
                let time_str = if config.clock_24hr {
                    format!("{:02}:{:02}:{:02}", st.wHour, st.wMinute, st.wSecond)
                } else {
                    let (h, ampm) = if st.wHour == 0 {
                        (12, "AM")
                    } else if st.wHour > 12 {
                        (st.wHour - 12, "PM")
                    } else if st.wHour == 12 {
                        (12, "PM")
                    } else {
                        (st.wHour, "AM")
                    };
                    format!("{:02}:{:02}:{:02} {}", h, st.wMinute, st.wSecond, ampm)
                };

                SelectObject(hdc, ctx.hfont_clock);
                SetTextColor(hdc, ctx.pal.accent_cyan);
                ctx.draw_text(hdc, x + 14, inside_y, &time_str);
                inside_y += ctx.lh(28);
            }

            if config.date_format != DateFormat::Disabled {
                let day_idx = (st.wDayOfWeek as usize).min(6);
                let mon_idx = (st.wMonth as usize).saturating_sub(1).min(11);

                let date_str = match config.date_format {
                    DateFormat::Short => {
                        format!("{:02}/{:02}/{:04}", st.wMonth, st.wDay, st.wYear)
                    }
                    DateFormat::Normal => {
                        format!("{}, {} {}", DAYS[day_idx], MONTHS[mon_idx], st.wDay)
                    }
                    DateFormat::Long => format!(
                        "{}, {} {}, {:04}",
                        DAYS_LONG[day_idx], MONTHS_LONG[mon_idx], st.wDay, st.wYear
                    ),
                    DateFormat::Disabled => String::new(),
                };

                SelectObject(hdc, ctx.hfont_label);
                SetTextColor(hdc, ctx.pal.text_primary);
                ctx.draw_text(hdc, x + 14, inside_y, &date_str);
            }
        }
    }
}
