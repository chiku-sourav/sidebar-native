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
    BatteryDeviceName, BatteryInformation, BatteryManufactureDate, BatteryManufactureName,
    BatteryTemperature, BatteryUniqueID, BATTERY_CHARGING, BATTERY_CRITICAL, BATTERY_DISCHARGING,
    BATTERY_INFORMATION, BATTERY_MANUFACTURE_DATE, BATTERY_QUERY_INFORMATION, BATTERY_STATUS,
    BATTERY_WAIT_STATUS, GUID_DEVICE_BATTERY, IOCTL_BATTERY_QUERY_INFORMATION,
    IOCTL_BATTERY_QUERY_STATUS, IOCTL_BATTERY_QUERY_TAG,
};
use windows::Win32::System::IO::DeviceIoControl;

use super::types::SingleBatteryInfo;

pub unsafe fn query_ioctl_batteries() -> Vec<SingleBatteryInfo> {
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
                    if let Some(info) = query_single_battery_handle(handle) {
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
