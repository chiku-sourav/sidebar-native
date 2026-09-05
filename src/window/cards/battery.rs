use windows::Win32::Graphics::Gdi::{SelectObject, SetTextColor, HDC};

use crate::config::{AppConfig, TemperatureUnit};
use crate::telemetry::TelemetrySnapshot;
use crate::window::cards::CardRenderer;
use crate::window::context::{estimate_wrapped_lines, RenderContext};

pub struct BatteryCard;

impl BatteryCard {
    pub fn new() -> Self {
        Self
    }
}

impl CardRenderer for BatteryCard {
    fn name(&self) -> &'static str {
        "Power & Battery"
    }

    fn is_enabled(&self, config: &AppConfig) -> bool {
        config.show_battery
    }

    fn calculate_height(&self, snapshot: &TelemetrySnapshot, config: &AppConfig) -> i32 {
        let scale = config.font_size.scale();
        let sidebar_w = config.sidebar_width.max(300);

        if !snapshot.battery.has_battery {
            if config.show_disabled_hardware {
                return (72.0 * scale).round() as i32;
            } else {
                return 0;
            }
        }

        // Calculate dynamic height based on text wrapping for hardware model name
        let dev_title = if !snapshot.battery.device_name.is_empty() {
            if !snapshot.battery.manufacturer.is_empty() {
                format!(
                    "{} {} • {}",
                    snapshot.battery.manufacturer,
                    snapshot.battery.device_name,
                    snapshot.battery.chemistry
                )
            } else {
                format!(
                    "{} • {}",
                    snapshot.battery.device_name, snapshot.battery.chemistry
                )
            }
        } else {
            format!("Internal Battery • {}", snapshot.battery.chemistry)
        };

        let name_lines = estimate_wrapped_lines(&dev_title, sidebar_w - 28, scale);
        let extra_name_h = name_lines.saturating_sub(1) as f32 * 18.0;

        let has_multi_bat = snapshot.battery.batteries.len() > 1;
        let extra_bat_h = if has_multi_bat {
            (snapshot.battery.batteries.len() as f32 * 22.0) + 10.0
        } else {
            0.0
        };

        let has_temp = snapshot.battery.temperature_c.is_some();
        let has_cycles = snapshot.battery.cycle_count.is_some();
        let has_health = snapshot.battery.health_percent.is_some();

        let mut base_h = if has_health || has_cycles || has_temp {
            208.0
        } else {
            176.0
        };
        if config.adv_battery {
            base_h += 60.0; // 3 extra advanced rows
        }

        ((base_h + extra_name_h + extra_bat_h) * scale).round() as i32
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
        if !snapshot.battery.has_battery && !config.show_disabled_hardware {
            return;
        }

        unsafe {
            let card_h = self.calculate_height(snapshot, config);
            ctx.draw_card(hdc, x, y, w, card_h, ctx.pal.bg_card, ctx.pal.card_border);

            let mut inside_y = y + ctx.lh(11);

            // 1. Header with Status Tag
            SelectObject(hdc, ctx.hfont_header);
            SetTextColor(hdc, ctx.pal.text_muted);

            let tag = if !snapshot.battery.has_battery {
                "[AC POWER / NO BATTERY]"
            } else if snapshot.battery.is_charging {
                if snapshot.battery.percentage >= 99 {
                    "[FULLY CHARGED]"
                } else {
                    "[CHARGING]"
                }
            } else if snapshot.battery.is_saver_active {
                "[SAVER ACTIVE]"
            } else if snapshot.battery.is_critical {
                "[CRITICAL LEVEL]"
            } else if snapshot.battery.is_ac_connected {
                "[AC CONNECTED]"
            } else {
                "[ON BATTERY]"
            };

            ctx.draw_text(hdc, x + 14, inside_y, &format!("POWER & BATTERY {}", tag));

            // If no battery installed on system (Desktop PC)
            if !snapshot.battery.has_battery {
                inside_y += ctx.lh(20);
                SelectObject(hdc, ctx.hfont_label);
                SetTextColor(hdc, ctx.pal.text_muted);
                ctx.draw_text(
                    hdc,
                    x + 14,
                    inside_y,
                    "Desktop PC • Running on Direct AC Power Supply",
                );

                inside_y += ctx.lh(20);
                ctx.draw_key_value(
                    hdc,
                    x + 14,
                    inside_y,
                    w - 28,
                    "Power Source",
                    "AC Wall Outlet (Continuous)",
                    ctx.pal.text_muted,
                    ctx.pal.accent_green,
                );
                return;
            }

            // 2. Hardware Model & Chemistry
            let dev_title = if !snapshot.battery.device_name.is_empty() {
                if !snapshot.battery.manufacturer.is_empty() {
                    format!(
                        "{} {} • {}",
                        snapshot.battery.manufacturer,
                        snapshot.battery.device_name,
                        snapshot.battery.chemistry
                    )
                } else {
                    format!(
                        "{} • {}",
                        snapshot.battery.device_name, snapshot.battery.chemistry
                    )
                }
            } else {
                format!("Internal Battery • {}", snapshot.battery.chemistry)
            };

            inside_y += ctx.lh(20);
            SelectObject(hdc, ctx.hfont_label);
            SetTextColor(hdc, ctx.pal.text_primary);
            let wrapped_lines = ctx.wrap_text(hdc, ctx.hfont_label, &dev_title, w - 28);
            for line in wrapped_lines {
                ctx.draw_text(hdc, x + 14, inside_y, &line);
                inside_y += ctx.lh(18);
            }

            inside_y += ctx.lh(4);

            // 3. Status Description & Percentage Level
            let pct_color = if snapshot.battery.is_charging || snapshot.battery.percentage >= 80 {
                ctx.pal.accent_green
            } else if snapshot.battery.percentage <= 20 {
                ctx.pal.accent_red
            } else if snapshot.battery.percentage <= 40 {
                ctx.pal.accent_amber
            } else {
                ctx.pal.accent_cyan
            };

            ctx.draw_key_value(
                hdc,
                x + 14,
                inside_y,
                w - 28,
                &snapshot.battery.power_state_description,
                &format!("{}%", snapshot.battery.percentage),
                ctx.pal.text_muted,
                pct_color,
            );

            // 4. Progress Bar
            inside_y += ctx.lh(22);
            let bar_w = w - 28;
            let fill_w =
                ((snapshot.battery.percentage as f32 / 100.0) * bar_w as f32).round() as i32;
            ctx.draw_progress_bar(
                hdc,
                x + 14,
                inside_y,
                bar_w,
                ctx.lh(7).max(5),
                fill_w,
                pct_color,
                ctx.pal.progress_track,
            );

            // 5. Estimated Time & Flow Rate (Watts)
            inside_y += ctx.lh(18);
            let rate_str = if let Some(rate) = snapshot.battery.rate_watts {
                if rate > 0.05 {
                    format!("+{:.2} W (Charge Rate)", rate)
                } else if rate < -0.05 {
                    format!("{:.2} W (Discharge)", rate)
                } else {
                    "0.00 W (Idle)".to_string()
                }
            } else {
                "Power Flow: AC/Bat".to_string()
            };

            let rate_col = if snapshot.battery.is_charging {
                ctx.pal.accent_green
            } else if snapshot.battery.is_discharging {
                ctx.pal.accent_amber
            } else {
                ctx.pal.text_muted
            };

            ctx.draw_key_value(
                hdc,
                x + 14,
                inside_y,
                w - 28,
                &snapshot.battery.time_remaining_formatted,
                &rate_str,
                ctx.pal.text_muted,
                rate_col,
            );

            // 6. Terminal Voltage, Energy Saver & Temperature
            inside_y += ctx.lh(20);
            let left_v_str = if let Some(volts) = snapshot.battery.voltage_volts {
                format!("{:.3} V Terminal", volts)
            } else {
                "Voltage: Standard".to_string()
            };

            let right_status_str = if let Some(temp_c) = snapshot.battery.temperature_c {
                match config.temperature_unit {
                    TemperatureUnit::Celsius => format!("{:.1} °C Cell Temp", temp_c),
                    TemperatureUnit::Fahrenheit => {
                        format!("{:.1} °F Cell Temp", (temp_c * 9.0 / 5.0) + 32.0)
                    }
                }
            } else if snapshot.battery.is_saver_active {
                "Battery Saver: Active".to_string()
            } else if !snapshot.battery.serial_number.is_empty() {
                format!("SN: {}", snapshot.battery.serial_number)
            } else {
                "Battery Saver: Off".to_string()
            };

            ctx.draw_key_value(
                hdc,
                x + 14,
                inside_y,
                w - 28,
                &left_v_str,
                &right_status_str,
                ctx.pal.text_muted,
                ctx.pal.text_primary,
            );

            // 7. Battery Energy Capacity (Stored vs Full vs Designed)
            inside_y += ctx.lh(20);
            let rem_wh = snapshot.battery.remaining_capacity_mwh as f32 / 1000.0;
            let full_wh = snapshot.battery.full_charge_capacity_mwh as f32 / 1000.0;
            let design_wh = snapshot.battery.designed_capacity_mwh as f32 / 1000.0;

            let capacity_val_str = if full_wh > 0.1 && design_wh > 0.1 {
                format!(
                    "{:.1} Wh / {:.1} Wh ({:.1} Wh Design)",
                    rem_wh, full_wh, design_wh
                )
            } else if full_wh > 0.1 {
                format!("{:.1} Wh / {:.1} Wh", rem_wh, full_wh)
            } else if snapshot.battery.full_charge_capacity_mwh > 0 {
                format!(
                    "{} mWh / {} mWh",
                    snapshot.battery.remaining_capacity_mwh,
                    snapshot.battery.full_charge_capacity_mwh
                )
            } else {
                "Standard Capacity".to_string()
            };

            ctx.draw_key_value(
                hdc,
                x + 14,
                inside_y,
                w - 28,
                "Energy Capacity",
                &capacity_val_str,
                ctx.pal.text_muted,
                ctx.pal.accent_cyan,
            );

            // 8. Health, Degradation Wear & Charge Cycles
            inside_y += ctx.lh(20);
            let health_str = if let Some(h) = snapshot.battery.health_percent {
                let wear_str = snapshot
                    .battery
                    .wear_percent
                    .map(|w| format!(" ({:.1}% Wear)", w))
                    .unwrap_or_default();
                format!("{:.1}% Health{}", h, wear_str)
            } else {
                "Good Condition".to_string()
            };

            let health_col = if let Some(h) = snapshot.battery.health_percent {
                if h >= 80.0 {
                    ctx.pal.accent_green
                } else if h >= 65.0 {
                    ctx.pal.accent_amber
                } else {
                    ctx.pal.accent_red
                }
            } else {
                ctx.pal.accent_green
            };

            let cycles_str = if let Some(cycles) = snapshot.battery.cycle_count {
                format!("{} Cycles", cycles)
            } else if let Some(mfg_date) = &snapshot.battery.manufacture_date {
                format!("Mfg: {}", mfg_date)
            } else {
                "Smart Battery".to_string()
            };

            ctx.draw_key_value(
                hdc,
                x + 14,
                inside_y,
                w - 28,
                &health_str,
                &cycles_str,
                health_col,
                ctx.pal.text_muted,
            );

            // 9. Dot Row for Energy Breakdown
            inside_y += ctx.lh(20);
            if full_wh > 0.1 && design_wh > 0.1 {
                ctx.draw_dot_row(
                    hdc,
                    x + 14,
                    inside_y,
                    &format!("Remaining ({:.1}Wh)", rem_wh),
                    pct_color,
                    &format!("Full ({:.1}Wh)", full_wh),
                    ctx.pal.accent_cyan,
                    &format!("Design ({:.1}Wh)", design_wh),
                    ctx.pal.text_muted,
                );
            }

            // Advanced Battery Details
            if config.adv_battery {
                inside_y += ctx.lh(22);
                let alerts_str = format!(
                    "{} • Alert: {} mWh",
                    snapshot.battery.power_plan_name, snapshot.battery.low_capacity_alert_mwh
                );
                ctx.draw_key_value(
                    hdc,
                    x + 14,
                    inside_y,
                    w - 28,
                    "Power Plan & Alerts",
                    &alerts_str,
                    ctx.pal.text_muted,
                    ctx.pal.text_primary,
                );

                inside_y += ctx.lh(20);
                let raw_mw_str = snapshot
                    .battery
                    .charge_rate_mw
                    .map(|r| format!("{} mW", r))
                    .unwrap_or_else(|| "0 mW".to_string());
                let gran_str = format!(
                    "Gran: {}/{} mWh • {}",
                    snapshot.battery.capacity_granularity_1_mwh,
                    snapshot.battery.capacity_granularity_2_mwh,
                    raw_mw_str
                );
                ctx.draw_key_value(
                    hdc,
                    x + 14,
                    inside_y,
                    w - 28,
                    "Granularity & Flow",
                    &gran_str,
                    ctx.pal.text_muted,
                    ctx.pal.accent_cyan,
                );

                let has_sn = !snapshot.battery.serial_number.is_empty();
                let has_mfg = snapshot.battery.manufacture_date.is_some();
                if has_sn || has_mfg {
                    inside_y += ctx.lh(20);
                    let sn_part = if has_sn {
                        format!("SN: {}", snapshot.battery.serial_number)
                    } else {
                        String::new()
                    };
                    let mfg_part = if let Some(mfg) = &snapshot.battery.manufacture_date {
                        format!("Mfg: {}", mfg)
                    } else {
                        String::new()
                    };
                    let sn_mfg_str = if has_sn && has_mfg {
                        format!("{} • {}", sn_part, mfg_part)
                    } else {
                        format!("{}{}", sn_part, mfg_part)
                    };
                    ctx.draw_key_value(
                        hdc,
                        x + 14,
                        inside_y,
                        w - 28,
                        "Hardware Identity",
                        &sn_mfg_str,
                        ctx.pal.text_muted,
                        ctx.pal.text_muted,
                    );
                }
            }

            // 10. Multi-Battery Breakdown (if laptop has multiple packs)
            if snapshot.battery.batteries.len() > 1 {
                inside_y += ctx.lh(22);
                for (i, bat) in snapshot.battery.batteries.iter().enumerate() {
                    let b_rem_wh = bat.current_capacity_mwh as f32 / 1000.0;
                    let b_full_wh = bat.full_charge_capacity_mwh as f32 / 1000.0;
                    let b_pct = if bat.full_charge_capacity_mwh > 0 {
                        ((bat.current_capacity_mwh as f32 / bat.full_charge_capacity_mwh as f32)
                            * 100.0)
                            .round() as u8
                    } else {
                        snapshot.battery.percentage
                    };

                    let b_label = if !bat.name.is_empty() {
                        format!("Battery #{} ({})", i + 1, bat.name)
                    } else {
                        format!("Battery Pack #{}", i + 1)
                    };

                    let b_val = format!(
                        "{}% • {:.1}/{:.1} Wh{}",
                        b_pct,
                        b_rem_wh,
                        b_full_wh,
                        bat.cycle_count
                            .map(|c| format!(" ({} cyc)", c))
                            .unwrap_or_default()
                    );

                    ctx.draw_key_value(
                        hdc,
                        x + 14,
                        inside_y,
                        w - 28,
                        &b_label,
                        &b_val,
                        ctx.pal.text_muted,
                        ctx.pal.text_primary,
                    );
                    inside_y += ctx.lh(20);
                }
            }
        }
    }
}
