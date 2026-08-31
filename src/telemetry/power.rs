use std::ffi::c_void;
use windows::core::PCWSTR;
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
    SetupDiGetDeviceInterfaceDetailW, DIGCF_DEVICEINTERFACE, DIGCF_PRESENT,
    SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W,
};
use windows::Win32::Foundation::{
    CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Power::{
    BatteryDeviceName, BatteryEstimatedTime, BatteryGranularityInformation, BatteryInformation,
    BatteryManufactureDate, BatteryManufactureName, BatteryTemperature, BatteryUniqueID,
    CallNtPowerInformation, GetSystemPowerStatus, SystemBatteryState, BATTERY_CHARGING,
    BATTERY_CRITICAL, BATTERY_DISCHARGING, BATTERY_INFORMATION, BATTERY_MANUFACTURE_DATE,
    BATTERY_POWER_ON_LINE, BATTERY_QUERY_INFORMATION, BATTERY_QUERY_INFORMATION_LEVEL,
    BATTERY_STATUS, BATTERY_WAIT_STATUS, GUID_DEVICE_BATTERY, IOCTL_BATTERY_QUERY_INFORMATION,
    IOCTL_BATTERY_QUERY_STATUS, IOCTL_BATTERY_QUERY_TAG, SYSTEM_BATTERY_STATE, SYSTEM_POWER_STATUS,
};
use windows::Win32::System::IO::DeviceIoControl;

#[derive(Debug, Clone, Default)]
pub struct SingleBatteryInfo {
    pub name: String,
    pub manufacturer: String,
    pub serial_number: String,
    pub chemistry: String,
    pub manufacture_date: Option<String>,
    pub designed_capacity_mwh: u32,
    pub full_charge_capacity_mwh: u32,
    pub current_capacity_mwh: u32,
    pub health_percent: Option<f32>,
    pub wear_percent: Option<f32>,
    pub cycle_count: Option<u32>,
    pub voltage_volts: Option<f32>,
    pub rate_watts: Option<f32>,
    pub temperature_c: Option<f32>,
    pub is_charging: bool,
    pub is_discharging: bool,
    pub is_critical: bool,
}

#[derive(Debug, Clone, Default)]
pub struct BatteryMetrics {
    pub has_battery: bool,
    pub is_charging: bool,
    pub is_discharging: bool,
    pub is_ac_connected: bool,
    pub is_saver_active: bool,
    pub is_critical: bool,
    pub percentage: u8,
    pub life_time_seconds: Option<u32>,
    pub time_remaining_formatted: String,

    // Aggregate capacities & rates
    pub remaining_capacity_mwh: u32,
    pub full_charge_capacity_mwh: u32,
    pub designed_capacity_mwh: u32,
    pub health_percent: Option<f32>,
    pub wear_percent: Option<f32>,
    pub cycle_count: Option<u32>,
    pub rate_watts: Option<f32>,
    pub voltage_volts: Option<f32>,
    pub temperature_c: Option<f32>,

    // Hardware metadata
    pub chemistry: String,
    pub device_name: String,
    pub manufacturer: String,
    pub serial_number: String,
    pub manufacture_date: Option<String>,
    pub power_state_description: String,

    // Multi-battery list
    pub batteries: Vec<SingleBatteryInfo>,
}

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
            // 1. Basic Win32 Power Status
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

            // 2. CallNtPowerInformation for SystemBatteryState
            let mut sbs = SYSTEM_BATTERY_STATE::default();
            let nt_res = CallNtPowerInformation(
                SystemBatteryState,
                None,
                0,
                Some(&mut sbs as *mut _ as *mut c_void),
                std::mem::size_of::<SYSTEM_BATTERY_STATE>() as u32,
            );

            let has_nt_battery = nt_res.is_ok() && sbs.BatteryPresent.as_bool();
            let has_battery = has_battery_flag || has_nt_battery;

            if !has_battery {
                return BatteryMetrics {
                    has_battery: false,
                    is_ac_connected,
                    is_saver_active,
                    power_state_description: if is_ac_connected {
                        "AC Power (No Battery)".to_string()
                    } else {
                        "No Battery".to_string()
                    },
                    ..Default::default()
                };
            }

            // 3. Query Device IOCTL for detailed hardware info, health, cycles, voltage, chemistry, etc.
            let individual_batteries = self.query_ioctl_batteries();

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

            for bat in &individual_batteries {
                total_remaining_mwh += bat.current_capacity_mwh;
                total_full_mwh += bat.full_charge_capacity_mwh;
                total_design_mwh += bat.designed_capacity_mwh;

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
            if total_remaining_mwh == 0 && nt_res.is_ok() && sbs.RemainingCapacity > 0 {
                total_remaining_mwh = sbs.RemainingCapacity;
            }
            if total_full_mwh == 0 && nt_res.is_ok() && sbs.MaxCapacity > 0 {
                total_full_mwh = sbs.MaxCapacity;
            }

            // Fallback rate from NtPowerInformation if IOCTL didn't return one
            if agg_rate_watts.is_none() && nt_res.is_ok() && sbs.Rate != 0 {
                let rate_mw = sbs.Rate; // negative = discharging, positive = charging in mW
                agg_rate_watts = Some(rate_mw as f32 / 1000.0);
            }

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
            let is_charging = is_charging_status
                || (nt_res.is_ok() && sbs.Charging.as_bool())
                || individual_batteries.iter().any(|b| b.is_charging)
                || agg_rate_watts.map(|r| r > 0.05).unwrap_or(false);

            let is_discharging = !is_ac_connected
                || (nt_res.is_ok() && sbs.Discharging.as_bool())
                || individual_batteries.iter().any(|b| b.is_discharging)
                || agg_rate_watts.map(|r| r < -0.05).unwrap_or(false);

            let is_critical =
                is_critical_status || individual_batteries.iter().any(|b| b.is_critical);

            // Compute Percentage
            let percentage = if total_full_mwh > 0 && total_remaining_mwh > 0 {
                let calc_pct =
                    ((total_remaining_mwh as f64 / total_full_mwh as f64) * 100.0).round() as u8;
                calc_pct.clamp(0, 100)
            } else {
                pct_status
            };

            // Health calculation: Full Capacity / Design Capacity
            let (health_percent, wear_percent) = if total_design_mwh > 0 && total_full_mwh > 0 {
                let h = (total_full_mwh as f32 / total_design_mwh as f32) * 100.0;
                let clamped_h = h.min(100.0);
                let wear = (100.0 - clamped_h).max(0.0);
                (Some(clamped_h), Some(wear))
            } else {
                (None, None)
            };

            // Estimated Remaining / Charging Time
            let life_time_seconds = if sys_lifetime_sec.is_some() && sys_lifetime_sec != Some(0) {
                sys_lifetime_sec
            } else if nt_res.is_ok() && sbs.EstimatedTime != 0 && sbs.EstimatedTime != u32::MAX {
                Some(sbs.EstimatedTime)
            } else if is_charging
                && agg_rate_watts.map(|r| r > 0.5).unwrap_or(false)
                && total_full_mwh > total_remaining_mwh
            {
                // Approximate time to full charge: (Remaining mWh to full) / (Rate in mW) * 3600
                let needed_mwh = (total_full_mwh - total_remaining_mwh) as f32;
                let rate_mw = agg_rate_watts.unwrap() * 1000.0;
                if rate_mw > 100.0 {
                    let sec = ((needed_mwh / rate_mw) * 3600.0).round() as u32;
                    Some(sec)
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
                    let sec = ((rem_mwh / rate_mw) * 3600.0).round() as u32;
                    Some(sec)
                } else {
                    None
                }
            } else {
                None
            };

            let time_remaining_formatted = if let Some(secs) = life_time_seconds {
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
            };

            // Power State Description
            let power_state_description = if is_charging {
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
            };

            BatteryMetrics {
                has_battery: true,
                is_charging,
                is_discharging,
                is_ac_connected,
                is_saver_active,
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
                batteries: individual_batteries,
            }
        }
    }

    unsafe fn query_ioctl_batteries(&self) -> Vec<SingleBatteryInfo> {
        let mut results = Vec::new();

        let hdevinfo = match SetupDiGetClassDevsW(
            Some(&GUID_DEVICE_BATTERY),
            None,
            None,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        ) {
            Ok(h) => h,
            Err(_) => return results,
        };

        let mut iface_data = SP_DEVICE_INTERFACE_DATA {
            cbSize: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
            ..Default::default()
        };

        let mut member_index = 0u32;
        while SetupDiEnumDeviceInterfaces(
            hdevinfo,
            None,
            &GUID_DEVICE_BATTERY,
            member_index,
            &mut iface_data,
        )
        .is_ok()
        {
            member_index += 1;

            let mut required_size = 0u32;
            let _ = SetupDiGetDeviceInterfaceDetailW(
                hdevinfo,
                &iface_data,
                None,
                0,
                Some(&mut required_size),
                None,
            );

            if required_size == 0 {
                continue;
            }

            let mut buffer = vec![0u8; required_size as usize];
            let detail = buffer.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
            (*detail).cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;

            if SetupDiGetDeviceInterfaceDetailW(
                hdevinfo,
                &iface_data,
                Some(detail),
                required_size,
                None,
                None,
            )
            .is_ok()
            {
                let device_path_ptr = &(*detail).DevicePath as *const [u16; 1] as *const u16;
                let device_path = PCWSTR(device_path_ptr);

                let handle = CreateFileW(
                    device_path,
                    (GENERIC_READ | GENERIC_WRITE).0,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    None,
                    OPEN_EXISTING,
                    windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
                    HANDLE(std::ptr::null_mut()),
                );

                if let Ok(handle) = handle {
                    if handle != INVALID_HANDLE_VALUE {
                        if let Some(info) = Self::query_single_battery_handle(handle) {
                            results.push(info);
                        }
                        let _ = CloseHandle(handle);
                    }
                }
            }
        }

        let _ = SetupDiDestroyDeviceInfoList(hdevinfo);
        results
    }

    unsafe fn query_single_battery_handle(handle: HANDLE) -> Option<SingleBatteryInfo> {
        // 1. Get Battery Tag
        let mut tag: u32 = 0;
        let dw_wait: u32 = 0;
        let mut bytes_returned = 0u32;
        let res = DeviceIoControl(
            handle,
            IOCTL_BATTERY_QUERY_TAG,
            Some(&dw_wait as *const _ as *const c_void),
            std::mem::size_of::<u32>() as u32,
            Some(&mut tag as *mut _ as *mut c_void),
            std::mem::size_of::<u32>() as u32,
            Some(&mut bytes_returned),
            None,
        );

        if res.is_err() || tag == 0 {
            return None;
        }

        let mut battery_info = SingleBatteryInfo::default();

        // 2. Query Battery Information (Capacity, Chemistry, Cycles)
        let mut bqi = BATTERY_QUERY_INFORMATION {
            BatteryTag: tag,
            InformationLevel: BatteryInformation,
            AtRate: 0,
        };
        let mut bi = BATTERY_INFORMATION::default();
        if DeviceIoControl(
            handle,
            IOCTL_BATTERY_QUERY_INFORMATION,
            Some(&bqi as *const _ as *const c_void),
            std::mem::size_of::<BATTERY_QUERY_INFORMATION>() as u32,
            Some(&mut bi as *mut _ as *mut c_void),
            std::mem::size_of::<BATTERY_INFORMATION>() as u32,
            Some(&mut bytes_returned),
            None,
        )
        .is_ok()
        {
            battery_info.designed_capacity_mwh = bi.DesignedCapacity;
            battery_info.full_charge_capacity_mwh = bi.FullChargedCapacity;
            if bi.CycleCount > 0 && bi.CycleCount != u32::MAX {
                battery_info.cycle_count = Some(bi.CycleCount);
            }

            // Chemistry decoding
            let chem_bytes = bi.Chemistry;
            let chem_str = String::from_utf8_lossy(&chem_bytes)
                .trim_matches('\0')
                .trim()
                .to_string();
            battery_info.chemistry = match chem_str.as_str() {
                "LION" | "Li-I" => "Lithium-Ion (Li-Ion)".to_string(),
                "LiP" | "LIP" => "Lithium-Polymer (Li-Poly)".to_string(),
                "NiMH" => "Nickel-Metal Hydride (NiMH)".to_string(),
                "NiCd" => "Nickel-Cadmium (NiCd)".to_string(),
                "PbAc" => "Lead-Acid (PbAc)".to_string(),
                other if !other.is_empty() => other.to_string(),
                _ => "Lithium-Ion".to_string(),
            };

            if bi.DesignedCapacity > 0 && bi.FullChargedCapacity > 0 {
                let h = (bi.FullChargedCapacity as f32 / bi.DesignedCapacity as f32) * 100.0;
                let clamped_h = h.min(100.0);
                battery_info.health_percent = Some(clamped_h);
                battery_info.wear_percent = Some((100.0 - clamped_h).max(0.0));
            }
        }

        // 3. Query Device Model Name
        bqi.InformationLevel = BatteryDeviceName;
        let mut name_buf = [0u16; 128];
        if DeviceIoControl(
            handle,
            IOCTL_BATTERY_QUERY_INFORMATION,
            Some(&bqi as *const _ as *const c_void),
            std::mem::size_of::<BATTERY_QUERY_INFORMATION>() as u32,
            Some(name_buf.as_mut_ptr() as *mut c_void),
            (name_buf.len() * 2) as u32,
            Some(&mut bytes_returned),
            None,
        )
        .is_ok()
        {
            let len = (bytes_returned as usize / 2).min(name_buf.len());
            let name = String::from_utf16_lossy(&name_buf[..len])
                .trim_matches('\0')
                .trim()
                .to_string();
            if !name.is_empty() {
                battery_info.name = name;
            }
        }

        // 4. Query Manufacturer Name
        bqi.InformationLevel = BatteryManufactureName;
        let mut mfg_buf = [0u16; 128];
        if DeviceIoControl(
            handle,
            IOCTL_BATTERY_QUERY_INFORMATION,
            Some(&bqi as *const _ as *const c_void),
            std::mem::size_of::<BATTERY_QUERY_INFORMATION>() as u32,
            Some(mfg_buf.as_mut_ptr() as *mut c_void),
            (mfg_buf.len() * 2) as u32,
            Some(&mut bytes_returned),
            None,
        )
        .is_ok()
        {
            let len = (bytes_returned as usize / 2).min(mfg_buf.len());
            let mfg = String::from_utf16_lossy(&mfg_buf[..len])
                .trim_matches('\0')
                .trim()
                .to_string();
            if !mfg.is_empty() {
                battery_info.manufacturer = mfg;
            }
        }

        // 5. Query Unique ID / Serial Number
        bqi.InformationLevel = BatteryUniqueID;
        let mut uid_buf = [0u16; 128];
        if DeviceIoControl(
            handle,
            IOCTL_BATTERY_QUERY_INFORMATION,
            Some(&bqi as *const _ as *const c_void),
            std::mem::size_of::<BATTERY_QUERY_INFORMATION>() as u32,
            Some(uid_buf.as_mut_ptr() as *mut c_void),
            (uid_buf.len() * 2) as u32,
            Some(&mut bytes_returned),
            None,
        )
        .is_ok()
        {
            let len = (bytes_returned as usize / 2).min(uid_buf.len());
            let uid = String::from_utf16_lossy(&uid_buf[..len])
                .trim_matches('\0')
                .trim()
                .to_string();
            if !uid.is_empty() {
                battery_info.serial_number = uid;
            }
        }

        // 6. Query Manufacture Date
        bqi.InformationLevel = BatteryManufactureDate;
        let mut mfg_date = BATTERY_MANUFACTURE_DATE::default();
        if DeviceIoControl(
            handle,
            IOCTL_BATTERY_QUERY_INFORMATION,
            Some(&bqi as *const _ as *const c_void),
            std::mem::size_of::<BATTERY_QUERY_INFORMATION>() as u32,
            Some(&mut mfg_date as *mut _ as *mut c_void),
            std::mem::size_of::<BATTERY_MANUFACTURE_DATE>() as u32,
            Some(&mut bytes_returned),
            None,
        )
        .is_ok()
        {
            if mfg_date.Year > 1990 && mfg_date.Month >= 1 && mfg_date.Month <= 12 {
                battery_info.manufacture_date = Some(format!(
                    "{:04}-{:02}-{:02}",
                    mfg_date.Year, mfg_date.Month, mfg_date.Day
                ));
            }
        }

        // 7. Query Battery Temperature
        bqi.InformationLevel = BatteryTemperature;
        let mut temp_k10 = 0u32;
        if DeviceIoControl(
            handle,
            IOCTL_BATTERY_QUERY_INFORMATION,
            Some(&bqi as *const _ as *const c_void),
            std::mem::size_of::<BATTERY_QUERY_INFORMATION>() as u32,
            Some(&mut temp_k10 as *mut _ as *mut c_void),
            std::mem::size_of::<u32>() as u32,
            Some(&mut bytes_returned),
            None,
        )
        .is_ok()
            && temp_k10 > 2000
            && temp_k10 < 4000
        {
            let temp_c = (temp_k10 as f32 / 10.0) - 273.15;
            battery_info.temperature_c = Some(temp_c);
        }

        // 8. Query Battery Real-Time Status (Voltage, Power State, Rate, Remaining Capacity)
        let bws = BATTERY_WAIT_STATUS {
            BatteryTag: tag,
            Timeout: 0,
            PowerState: 0,
            LowCapacity: 0,
            HighCapacity: 0,
        };
        let mut bs = BATTERY_STATUS::default();
        if DeviceIoControl(
            handle,
            IOCTL_BATTERY_QUERY_STATUS,
            Some(&bws as *const _ as *const c_void),
            std::mem::size_of::<BATTERY_WAIT_STATUS>() as u32,
            Some(&mut bs as *mut _ as *mut c_void),
            std::mem::size_of::<BATTERY_STATUS>() as u32,
            Some(&mut bytes_returned),
            None,
        )
        .is_ok()
        {
            battery_info.current_capacity_mwh = bs.Capacity;
            if bs.Voltage > 0 && bs.Voltage != u32::MAX {
                battery_info.voltage_volts = Some(bs.Voltage as f32 / 1000.0);
            }
            if bs.Rate != 0 && bs.Rate != i32::MIN && bs.Rate != i32::MAX {
                battery_info.rate_watts = Some(bs.Rate as f32 / 1000.0);
            }
            battery_info.is_charging = (bs.PowerState & BATTERY_CHARGING) != 0;
            battery_info.is_discharging = (bs.PowerState & BATTERY_DISCHARGING) != 0;
            battery_info.is_critical = (bs.PowerState & BATTERY_CRITICAL) != 0;
        }

        Some(battery_info)
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
