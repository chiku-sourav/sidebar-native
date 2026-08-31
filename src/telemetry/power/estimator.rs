pub fn calculate_health_and_wear(
    design_mwh: u32,
    full_mwh: u32,
) -> (Option<f32>, Option<f32>) {
    if design_mwh > 0 && full_mwh > 0 {
        let h = (full_mwh as f32 / design_mwh as f32) * 100.0;
        let clamped_h = h.min(100.0);
        let wear = (100.0 - clamped_h).max(0.0);
        (Some(clamped_h), Some(wear))
    } else {
        (None, None)
    }
}

pub fn estimate_lifetime_seconds(
    sys_lifetime_sec: Option<u32>,
    nt_estimated_time: u32,
    is_charging: bool,
    is_discharging: bool,
    agg_rate_watts: Option<f32>,
    total_full_mwh: u32,
    total_remaining_mwh: u32,
) -> Option<u32> {
    if sys_lifetime_sec.is_some() && sys_lifetime_sec != Some(0) {
        sys_lifetime_sec
    } else if nt_estimated_time != 0 && nt_estimated_time != u32::MAX {
        Some(nt_estimated_time)
    } else if is_charging
        && agg_rate_watts.map(|r| r > 0.5).unwrap_or(false)
        && total_full_mwh > total_remaining_mwh
    {
        // Approximate time to full charge: (Remaining mWh to full) / (Rate in mW) * 3600
        let needed_mwh = (total_full_mwh - total_remaining_mwh) as f32;
        let rate_mw = agg_rate_watts.unwrap() * 1000.0;
        if rate_mw > 100.0 {
            Some(((needed_mwh / rate_mw) * 3600.0).round() as u32)
        } else {
            None
        }
    } else if is_discharging
        && agg_rate_watts.map(|r| r < -0.5).unwrap_or(false)
        && total_remaining_mwh > 0
    {
        // Approximate time to empty: (Remaining mWh) / (|Rate in mW|) * 3600
        let rem_mwh = total_remaining_mwh as f32;
        let rate_mw = agg_rate_watts.unwrap().abs() * 1000.0;
        if rate_mw > 100.0 {
            Some(((rem_mwh / rate_mw) * 3600.0).round() as u32)
        } else {
            None
        }
    } else {
        None
    }
}

pub fn format_time_remaining(
    life_time_seconds: Option<u32>,
    is_charging: bool,
    is_ac_connected: bool,
    percentage: u8,
) -> String {
    if let Some(secs) = life_time_seconds {
        let hours = secs / 3600;
        let minutes = (secs % 3600) / 60;
        if is_charging {
            if hours > 0 {
                format!("{}h {}m until full", hours, minutes)
            } else if minutes > 0 {
                format!("{}m until full", minutes)
            } else {
                "Almost full".to_string()
            }
        } else {
            if hours > 0 {
                format!("{}h {}m remaining", hours, minutes)
            } else if minutes > 0 {
                format!("{}m remaining", minutes)
            } else {
                "< 1m remaining".to_string()
            }
        }
    } else if is_charging {
        if percentage >= 99 {
            "Fully Charged (Plugged in)".to_string()
        } else {
            "Charging...".to_string()
        }
    } else if is_ac_connected {
        "Plugged in (Not charging)".to_string()
    } else {
        "Calculating remaining time...".to_string()
    }
}

pub fn describe_power_state(
    is_charging: bool,
    is_ac_connected: bool,
    is_saver_active: bool,
    is_critical: bool,
    percentage: u8,
) -> String {
    if is_charging {
        if percentage >= 99 {
            "Plugged in (Fully Charged)".to_string()
        } else {
            "Plugged in (Charging)".to_string()
        }
    } else if is_ac_connected {
        "Plugged in (AC Power)".to_string()
    } else if is_saver_active {
        "On Battery (Battery Saver)".to_string()
    } else if is_critical {
        "On Battery (Critical Level)".to_string()
    } else {
        "On Battery (Discharging)".to_string()
    }
}

