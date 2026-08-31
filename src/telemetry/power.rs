use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

#[derive(Debug, Clone, Default)]
pub struct BatteryMetrics {
    pub has_battery: bool,
    pub is_charging: bool,
    pub is_ac_connected: bool,
    pub percentage: u8,
    pub life_time_seconds: u32,
}

pub struct PowerCollector;

impl PowerCollector {
    pub fn new() -> Self {
        Self
    }

    pub fn collect(&self) -> BatteryMetrics {
        unsafe {
            let mut status = SYSTEM_POWER_STATUS::default();
            if GetSystemPowerStatus(&mut status).is_ok() {
                let has_battery = status.BatteryFlag != 128 && status.BatteryLifePercent <= 100;
                let is_ac_connected = status.ACLineStatus == 1;
                let is_charging = (status.BatteryFlag & 8) != 0;
                let percentage = if has_battery {
                    status.BatteryLifePercent
                } else {
                    100
                };

                let life_time_seconds = if status.BatteryLifeTime != u32::MAX {
                    status.BatteryLifeTime
                } else {
                    0
                };

                BatteryMetrics {
                    has_battery,
                    is_charging,
                    is_ac_connected,
                    percentage,
                    life_time_seconds,
                }
            } else {
                BatteryMetrics::default()
            }
        }
    }
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
