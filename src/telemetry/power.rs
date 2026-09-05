pub mod estimator;
pub mod ioctl;
pub mod system;
pub mod types;

pub use types::{BatteryMetrics, SingleBatteryInfo};

use estimator::{
    calculate_health_and_wear, describe_power_state, estimate_lifetime_seconds,
    format_time_remaining,
};
use ioctl::query_ioctl_batteries;
use system::query_system_power;

pub struct PowerCollector {
    cached_designed_capacity: u32,
    cached_device_name: String,
    cached_manufacturer: String,
    cached_serial: String,
    cached_chemistry: String,
    cached_mfg_date: Option<String>,
}

impl PowerCollector {
    pub fn new() -> Self {
        Self {
            cached_designed_capacity: 0,
            cached_device_name: String::new(),
            cached_manufacturer: String::new(),
            cached_serial: String::new(),
            cached_chemistry: String::new(),
            cached_mfg_date: None,
        }
    }

    pub fn collect(&mut self) -> BatteryMetrics {
        unsafe {
            // 1. Basic Win32 Power Status and NtPowerInformation
            let sys = query_system_power();
            let has_battery = sys.has_battery_flag || sys.has_nt_battery;

            if !has_battery {
                return BatteryMetrics {
                    has_battery: false,
                    is_ac_connected: sys.is_ac_connected,
                    is_saver_active: sys.is_saver_active,
                    power_state_description: if sys.is_ac_connected {
                        "AC Power (No Battery)".to_string()
                    } else {
                        "No Battery".to_string()
                    },
                    ..Default::default()
                };
            }

            // 2. Query Device IOCTL for detailed hardware info, health, cycles, voltage, chemistry, etc.
            let individual_batteries = query_ioctl_batteries();

            let mut total_remaining_mwh = 0u32;
            let mut total_full_mwh = 0u32;
            let mut total_design_mwh = 0u32;
            let mut agg_rate_watts: Option<f32> = None;
            let mut agg_voltage_v: Option<f32> = None;
            let mut agg_temp_c: Option<f32> = None;
            let mut agg_cycle_count: Option<u32> = None;
            let mut agg_chem = String::new();
            let mut agg_dev_name = String::new();
            let mut agg_mfg = String::new();
            let mut agg_serial = String::new();
            let mut agg_mfg_date: Option<String> = None;

            let mut agg_low_alert = 0u32;
            let mut agg_warn_alert = 0u32;
            let mut agg_gran1 = 100u32;
            let mut agg_gran2 = 100u32;

            for bat in &individual_batteries {
                total_remaining_mwh += bat.current_capacity_mwh;
                total_full_mwh += bat.full_charge_capacity_mwh;
                total_design_mwh += bat.designed_capacity_mwh;

                if bat.low_capacity_alert_mwh > 0 {
                    agg_low_alert = bat.low_capacity_alert_mwh;
                }
                if bat.warning_capacity_alert_mwh > 0 {
                    agg_warn_alert = bat.warning_capacity_alert_mwh;
                }
                if bat.capacity_granularity_1_mwh > 0 {
                    agg_gran1 = bat.capacity_granularity_1_mwh;
                }
                if bat.capacity_granularity_2_mwh > 0 {
                    agg_gran2 = bat.capacity_granularity_2_mwh;
                }

                if let Some(r) = bat.rate_watts {
                    let current = agg_rate_watts.unwrap_or(0.0);
                    agg_rate_watts = Some(current + r);
                }
                if bat.voltage_volts.is_some() && agg_voltage_v.is_none() {
                    agg_voltage_v = bat.voltage_volts;
                }
                if bat.temperature_c.is_some() && agg_temp_c.is_none() {
                    agg_temp_c = bat.temperature_c;
                }
                if bat.cycle_count.is_some() && agg_cycle_count.is_none() {
                    agg_cycle_count = bat.cycle_count;
                }
                if !bat.chemistry.is_empty() && agg_chem.is_empty() {
                    agg_chem = bat.chemistry.clone();
                }
                if !bat.name.is_empty() && agg_dev_name.is_empty() {
                    agg_dev_name = bat.name.clone();
                }
                if !bat.manufacturer.is_empty() && agg_mfg.is_empty() {
                    agg_mfg = bat.manufacturer.clone();
                }
                if !bat.serial_number.is_empty() && agg_serial.is_empty() {
                    agg_serial = bat.serial_number.clone();
                }
                if bat.manufacture_date.is_some() && agg_mfg_date.is_none() {
                    agg_mfg_date = bat.manufacture_date.clone();
                }
            }

            // If IOCTL gave 0 for capacity, fall back to NtPowerInformation
            if total_remaining_mwh == 0 && sys.nt_remaining_mwh > 0 {
                total_remaining_mwh = sys.nt_remaining_mwh;
            }
            if total_full_mwh == 0 && sys.nt_max_mwh > 0 {
                total_full_mwh = sys.nt_max_mwh;
            }

            // Fallback rate from NtPowerInformation if IOCTL didn't return one
            if agg_rate_watts.is_none() && sys.nt_rate_mw != 0 {
                agg_rate_watts = Some(sys.nt_rate_mw as f32 / 1000.0);
            }

            let charge_rate_mw = agg_rate_watts.map(|w| (w * 1000.0) as i32);

            // Caching static info
            if total_design_mwh > 0 {
                self.cached_designed_capacity = total_design_mwh;
            } else if self.cached_designed_capacity > 0 {
                total_design_mwh = self.cached_designed_capacity;
            }

            if !agg_dev_name.is_empty() {
                self.cached_device_name = agg_dev_name.clone();
            } else if !self.cached_device_name.is_empty() {
                agg_dev_name = self.cached_device_name.clone();
            }

            if !agg_mfg.is_empty() {
                self.cached_manufacturer = agg_mfg.clone();
            } else if !self.cached_manufacturer.is_empty() {
                agg_mfg = self.cached_manufacturer.clone();
            }

            if !agg_serial.is_empty() {
                self.cached_serial = agg_serial.clone();
            } else if !self.cached_serial.is_empty() {
                agg_serial = self.cached_serial.clone();
            }

            if !agg_chem.is_empty() {
                self.cached_chemistry = agg_chem.clone();
            } else if !self.cached_chemistry.is_empty() {
                agg_chem = self.cached_chemistry.clone();
            } else {
                agg_chem = "Lithium-Ion".to_string();
            }

            if agg_mfg_date.is_some() {
                self.cached_mfg_date = agg_mfg_date.clone();
            } else if self.cached_mfg_date.is_some() {
                agg_mfg_date = self.cached_mfg_date.clone();
            }

            // Determine charging and discharging status
            let is_charging = sys.is_charging_status
                || sys.nt_charging
                || individual_batteries.iter().any(|b| b.is_charging)
                || agg_rate_watts.map(|r| r > 0.05).unwrap_or(false);

            let is_discharging = !sys.is_ac_connected
                || sys.nt_discharging
                || individual_batteries.iter().any(|b| b.is_discharging)
                || agg_rate_watts.map(|r| r < -0.05).unwrap_or(false);

            let is_critical =
                sys.is_critical_status || individual_batteries.iter().any(|b| b.is_critical);

            // Compute Percentage
            let percentage = if total_full_mwh > 0 && total_remaining_mwh > 0 {
                let calc_pct =
                    ((total_remaining_mwh as f64 / total_full_mwh as f64) * 100.0).round() as u8;
                calc_pct.clamp(0, 100)
            } else {
                sys.pct_status
            };

            // Health calculation: Full Capacity / Design Capacity
            let (health_percent, wear_percent) =
                calculate_health_and_wear(total_design_mwh, total_full_mwh);

            // Estimated Remaining / Charging Time
            let life_time_seconds = estimate_lifetime_seconds(
                sys.sys_lifetime_sec,
                sys.nt_estimated_time,
                is_charging,
                is_discharging,
                agg_rate_watts,
                total_full_mwh,
                total_remaining_mwh,
            );

            let time_remaining_formatted = format_time_remaining(
                life_time_seconds,
                is_charging,
                sys.is_ac_connected,
                percentage,
            );

            let power_state_description = describe_power_state(
                is_charging,
                sys.is_ac_connected,
                sys.is_saver_active,
                is_critical,
                percentage,
            );

            let power_plan_name = query_active_power_plan_name();

            BatteryMetrics {
                has_battery: true,
                is_charging,
                is_discharging,
                is_ac_connected: sys.is_ac_connected,
                is_saver_active: sys.is_saver_active,
                is_critical,
                percentage,
                life_time_seconds,
                time_remaining_formatted,
                remaining_capacity_mwh: total_remaining_mwh,
                full_charge_capacity_mwh: total_full_mwh,
                designed_capacity_mwh: total_design_mwh,
                health_percent,
                wear_percent,
                cycle_count: agg_cycle_count,
                rate_watts: agg_rate_watts,
                voltage_volts: agg_voltage_v,
                temperature_c: agg_temp_c,
                chemistry: agg_chem,
                device_name: agg_dev_name,
                manufacturer: agg_mfg,
                serial_number: agg_serial,
                manufacture_date: agg_mfg_date,
                power_state_description,
                charge_rate_mw,
                capacity_granularity_1_mwh: agg_gran1,
                capacity_granularity_2_mwh: agg_gran2,
                low_capacity_alert_mwh: agg_low_alert,
                warning_capacity_alert_mwh: agg_warn_alert,
                power_plan_name,
                batteries: individual_batteries,
            }
        }
    }
}

fn query_active_power_plan_name() -> String {
    use windows::core::w;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ,
    };

    unsafe {
        let key_path = w!("SYSTEM\\CurrentControlSet\\Control\\Power\\User\\PowerSchemes");
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, key_path, 0, KEY_READ, &mut hkey).is_ok() {
            let val_name = w!("ActivePowerScheme");
            let mut buf = [0u8; 128];
            let mut buf_len = buf.len() as u32;
            if RegQueryValueExW(
                hkey,
                val_name,
                None,
                None,
                Some(buf.as_mut_ptr()),
                Some(&mut buf_len),
            )
            .is_ok()
                && buf_len > 0
            {
                let u16_slice = std::slice::from_raw_parts(
                    buf.as_ptr() as *const u16,
                    (buf_len as usize / 2).saturating_sub(1),
                );
                let guid_str = String::from_utf16_lossy(u16_slice).to_lowercase();
                let _ = RegCloseKey(hkey);

                if guid_str.contains("381b4222") {
                    return "Balanced".to_string();
                } else if guid_str.contains("8c5e7fda") {
                    return "High Performance".to_string();
                } else if guid_str.contains("a1841308") {
                    return "Power Saver".to_string();
                } else if guid_str.contains("e9a42b02") {
                    return "Ultimate Performance".to_string();
                }
            }
            let _ = RegCloseKey(hkey);
        }
    }
    "Balanced".to_string()
}

impl super::collector::TelemetryCollector for PowerCollector {
    fn name(&self) -> &'static str {
        "Battery"
    }

    fn update(
        &mut self,
        snapshot: &mut super::TelemetrySnapshot,
        _config: &crate::config::AppConfig,
    ) {
        snapshot.battery = self.collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battery_collector() {
        let mut collector = PowerCollector::new();
        let metrics = collector.collect();
        println!("\n================ BATTERY METRICS ================");
        println!("Has battery: {}", metrics.has_battery);
        println!("State: {}", metrics.power_state_description);
        println!("AC connected: {}", metrics.is_ac_connected);
        println!("Charging: {}", metrics.is_charging);
        println!("Discharging: {}", metrics.is_discharging);
        println!("Saver active: {}", metrics.is_saver_active);
        println!("Percentage: {}%", metrics.percentage);
        println!(
            "Time remaining formatted: {}",
            metrics.time_remaining_formatted
        );
        println!(
            "Rate: {:?}",
            metrics.rate_watts.map(|w| format!("{:.2} W", w))
        );
        println!(
            "Voltage: {:?}",
            metrics.voltage_volts.map(|v| format!("{:.3} V", v))
        );
        println!("Designed capacity: {} mWh", metrics.designed_capacity_mwh);
        println!(
            "Full charge capacity: {} mWh",
            metrics.full_charge_capacity_mwh
        );
        println!("Remaining capacity: {} mWh", metrics.remaining_capacity_mwh);
        println!(
            "Health: {:?}",
            metrics.health_percent.map(|h| format!("{:.1}%", h))
        );
        println!(
            "Wear: {:?}",
            metrics.wear_percent.map(|w| format!("{:.1}%", w))
        );
        println!("Cycles: {:?}", metrics.cycle_count);
        println!("Chemistry: {}", metrics.chemistry);
        println!("Device Name: {}", metrics.device_name);
        println!("Manufacturer: {}", metrics.manufacturer);
        println!("Serial: {}", metrics.serial_number);
        println!("Mfg Date: {:?}", metrics.manufacture_date);
        println!(
            "Temperature: {:?}",
            metrics.temperature_c.map(|t| format!("{:.1} °C", t))
        );
        println!("Sub-batteries count: {}", metrics.batteries.len());
        for (i, b) in metrics.batteries.iter().enumerate() {
            println!("  [Battery #{}] Name: '{}', Mfg: '{}', Design: {} mWh, Full: {} mWh, Cur: {} mWh, Cycles: {:?}, Health: {:?}", 
                i+1, b.name, b.manufacturer, b.designed_capacity_mwh, b.full_charge_capacity_mwh, b.current_capacity_mwh, b.cycle_count, b.health_percent);
        }
        println!("=================================================\n");
    }
}
