use std::ffi::c_void;
use windows::Win32::System::Power::{
    CallNtPowerInformation, GetSystemPowerStatus, SystemBatteryState, SYSTEM_BATTERY_STATE,
    SYSTEM_POWER_STATUS,
};

#[derive(Debug, Clone, Default)]
pub struct SystemPowerSnapshot {
    pub has_status: bool,
    pub has_battery_flag: bool,
    pub is_ac_connected: bool,
    pub is_charging_status: bool,
    pub is_critical_status: bool,
    pub is_saver_active: bool,
    pub pct_status: u8,
    pub sys_lifetime_sec: Option<u32>,
    pub has_nt_battery: bool,
    pub nt_remaining_mwh: u32,
    pub nt_max_mwh: u32,
    pub nt_rate_mw: i32,
    pub nt_estimated_time: u32,
    pub nt_charging: bool,
    pub nt_discharging: bool,
}

pub unsafe fn query_system_power() -> SystemPowerSnapshot {
    let mut status = SYSTEM_POWER_STATUS::default();
    let has_status = GetSystemPowerStatus(&mut status).is_ok();

    let has_battery_flag = if has_status {
        status.BatteryFlag != 128 && status.BatteryLifePercent <= 100
    } else {
        false
    };

    let is_ac_connected = if has_status {
        status.ACLineStatus == 1
    } else {
        true
    };

    let is_charging_status = if has_status {
        (status.BatteryFlag & 8) != 0
    } else {
        false
    };

    let is_critical_status = if has_status {
        (status.BatteryFlag & 4) != 0
    } else {
        false
    };

    let is_saver_active = if has_status {
        status.SystemStatusFlag == 1
    } else {
        false
    };

    let pct_status = if has_status && has_battery_flag {
        status.BatteryLifePercent
    } else {
        100
    };

    let sys_lifetime_sec = if has_status && status.BatteryLifeTime != u32::MAX {
        Some(status.BatteryLifeTime)
    } else {
        None
    };

    let mut sbs = SYSTEM_BATTERY_STATE::default();
    let nt_res = CallNtPowerInformation(
        SystemBatteryState,
        None,
        0,
        Some(&mut sbs as *mut _ as *mut c_void),
        std::mem::size_of::<SYSTEM_BATTERY_STATE>() as u32,
    );

    let has_nt_battery = nt_res.is_ok() && sbs.BatteryPresent.as_bool();

    SystemPowerSnapshot {
        has_status,
        has_battery_flag,
        is_ac_connected,
        is_charging_status,
        is_critical_status,
        is_saver_active,
        pct_status,
        sys_lifetime_sec,
        has_nt_battery,
        nt_remaining_mwh: if nt_res.is_ok() { sbs.RemainingCapacity } else { 0 },
        nt_max_mwh: if nt_res.is_ok() { sbs.MaxCapacity } else { 0 },
        nt_rate_mw: if nt_res.is_ok() { sbs.Rate as i32 } else { 0 },
        nt_estimated_time: if nt_res.is_ok() { sbs.EstimatedTime } else { 0 },
        nt_charging: nt_res.is_ok() && sbs.Charging.as_bool(),
        nt_discharging: nt_res.is_ok() && sbs.Discharging.as_bool(),
    }
}

