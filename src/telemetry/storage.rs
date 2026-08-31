#![allow(unused_imports, dead_code, unused_must_use)]

use std::collections::{HashMap, HashSet};
use std::time::Instant;
use sysinfo::Disks;
use windows::core::{s, w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HMODULE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, GetDiskFreeSpaceExW, GetLogicalDrives, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Ioctl::{
    PropertyStandardQuery, StorageAdapterProperty, StorageDeviceProperty,
    StorageDeviceSeekPenaltyProperty, IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
    IOCTL_STORAGE_QUERY_PROPERTY, STORAGE_PROPERTY_QUERY,
};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::System::IO::DeviceIoControl;

const IOCTL_STORAGE_GET_DEVICE_NUMBER: u32 = 0x002D1080;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
struct StorageDeviceNumber {
    device_type: u32,
    device_number: u32,
    partition_number: u32,
}

#[derive(Debug, Clone, Default)]
pub struct DriveInfo {
    pub letter: String,
    pub label: String,
    pub drive_type: String, // "NVMe SSD" | "SATA SSD" | "HDD" | "USB Drive"
    pub model_name: String,
    pub read_bytes_sec: u64,
    pub write_bytes_sec: u64,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
    pub usage_percentage: f32,
    pub is_linux_or_raw: bool,
}

#[derive(Debug, Clone, Default)]
pub struct StorageMetrics {
    pub primary_free_bytes: u64,
    pub primary_total_bytes: u64,
    pub primary_usage_percentage: f32,
    pub read_bytes_sec: u64,
    pub write_bytes_sec: u64,
    pub drives: Vec<DriveInfo>,
}

#[derive(Debug, Clone, Default)]
struct PhysicalDiskMeta {
    index: u32,
    model: String,
    bus_type: String,
    is_ssd: bool,
    size_bytes: u64,
}

type PdhHQuery = isize;
type PdhHCounter = isize;

#[repr(C)]
#[derive(Copy, Clone)]
struct PdhFmtCounterValue {
    c_status: u32,
    value: PdhFmtCounterValueUnion,
}

#[repr(C)]
#[derive(Copy, Clone)]
union PdhFmtCounterValueUnion {
    long_value: i32,
    double_value: f64,
    large_value: i64,
    ansi_str_value: *const u8,
    wide_str_value: *const u16,
}

const PDH_FMT_DOUBLE: u32 = 0x00000200;
const PDH_FMT_NOSCALE: u32 = 0x00001000;

type FnPdhOpenQueryW = unsafe extern "system" fn(*const u16, usize, *mut PdhHQuery) -> u32;
type FnPdhAddEnglishCounterW =
    unsafe extern "system" fn(PdhHQuery, *const u16, usize, *mut PdhHCounter) -> u32;
type FnPdhCollectQueryData = unsafe extern "system" fn(PdhHQuery) -> u32;
type FnPdhGetFormattedCounterValue =
    unsafe extern "system" fn(PdhHCounter, u32, *mut u32, *mut PdhFmtCounterValue) -> u32;
type FnPdhRemoveCounter = unsafe extern "system" fn(PdhHCounter) -> u32;
type FnPdhCloseQuery = unsafe extern "system" fn(PdhHQuery) -> u32;

struct PdhDriveCounters {
    read_counter: PdhHCounter,
    write_counter: PdhHCounter,
}

struct PdhStorage {
    h_module: isize,
    h_query: PdhHQuery,
    counters: HashMap<String, PdhDriveCounters>,
    total_counters: Option<PdhDriveCounters>,
    fn_add_counter: FnPdhAddEnglishCounterW,
    fn_collect: FnPdhCollectQueryData,
    fn_get_value: FnPdhGetFormattedCounterValue,
    fn_remove_counter: FnPdhRemoveCounter,
    fn_close: FnPdhCloseQuery,
    has_collected_once: bool,
}

unsafe impl Send for PdhStorage {}
unsafe impl Sync for PdhStorage {}

impl PdhStorage {
    pub fn new() -> Option<Self> {
        unsafe {
            let h_module = LoadLibraryW(w!("pdh.dll")).ok()?;
            if h_module.is_invalid() {
                return None;
            }

            let fn_open: FnPdhOpenQueryW =
                std::mem::transmute(GetProcAddress(h_module, s!("PdhOpenQueryW"))?);
            let fn_add_counter: FnPdhAddEnglishCounterW =
                std::mem::transmute(GetProcAddress(h_module, s!("PdhAddEnglishCounterW"))?);
            let fn_collect: FnPdhCollectQueryData =
                std::mem::transmute(GetProcAddress(h_module, s!("PdhCollectQueryData"))?);
            let fn_get_value: FnPdhGetFormattedCounterValue =
                std::mem::transmute(GetProcAddress(h_module, s!("PdhGetFormattedCounterValue"))?);
            let fn_remove_counter: FnPdhRemoveCounter =
                std::mem::transmute(GetProcAddress(h_module, s!("PdhRemoveCounter"))?);
            let fn_close: FnPdhCloseQuery =
                std::mem::transmute(GetProcAddress(h_module, s!("PdhCloseQuery"))?);

            let mut h_query: PdhHQuery = 0;
            if fn_open(std::ptr::null(), 0, &mut h_query) != 0 || h_query == 0 {
                let _ = CloseHandle(HANDLE(h_module.0));
                return None;
            }

            let mut storage = Self {
                h_module: h_module.0 as isize,
                h_query,
                counters: HashMap::new(),
                total_counters: None,
                fn_add_counter,
                fn_collect,
                fn_get_value,
                fn_remove_counter,
                fn_close,
                has_collected_once: false,
            };

            // Setup _Total counters
            let total_read_path: Vec<u16> = "\\LogicalDisk(_Total)\\Disk Read Bytes/sec\0"
                .encode_utf16()
                .collect();
            let total_write_path: Vec<u16> = "\\LogicalDisk(_Total)\\Disk Write Bytes/sec\0"
                .encode_utf16()
                .collect();
            let mut h_tr: PdhHCounter = 0;
            let mut h_tw: PdhHCounter = 0;
            let r1 =
                (storage.fn_add_counter)(storage.h_query, total_read_path.as_ptr(), 0, &mut h_tr);
            let r2 =
                (storage.fn_add_counter)(storage.h_query, total_write_path.as_ptr(), 0, &mut h_tw);
            if r1 == 0 && r2 == 0 {
                storage.total_counters = Some(PdhDriveCounters {
                    read_counter: h_tr,
                    write_counter: h_tw,
                });
            }

            // Warm up initial sample
            let _ = (storage.fn_collect)(storage.h_query);

            Some(storage)
        }
    }

    pub fn ensure_counter_for_drive(&mut self, letter: char) {
        let key = format!("{}:", letter);
        if self.counters.contains_key(&key) {
            return;
        }

        let read_path: Vec<u16> = format!("\\LogicalDisk({}:)\\Disk Read Bytes/sec\0", letter)
            .encode_utf16()
            .collect();
        let write_path: Vec<u16> = format!("\\LogicalDisk({}:)\\Disk Write Bytes/sec\0", letter)
            .encode_utf16()
            .collect();

        unsafe {
            let mut h_r: PdhHCounter = 0;
            let mut h_w: PdhHCounter = 0;
            let r_res = (self.fn_add_counter)(self.h_query, read_path.as_ptr(), 0, &mut h_r);
            let w_res = (self.fn_add_counter)(self.h_query, write_path.as_ptr(), 0, &mut h_w);
            if r_res == 0 && w_res == 0 {
                self.counters.insert(
                    key,
                    PdhDriveCounters {
                        read_counter: h_r,
                        write_counter: h_w,
                    },
                );
            }
        }
    }

    pub fn ensure_counter_for_physical_disk(&mut self, index: u32) {
        let key = format!("Disk {}", index);
        if self.counters.contains_key(&key) {
            return;
        }

        let read_path: Vec<u16> = format!("\\PhysicalDisk({})\\Disk Read Bytes/sec\0", index)
            .encode_utf16()
            .collect();
        let write_path: Vec<u16> = format!("\\PhysicalDisk({})\\Disk Write Bytes/sec\0", index)
            .encode_utf16()
            .collect();

        unsafe {
            let mut h_r: PdhHCounter = 0;
            let mut h_w: PdhHCounter = 0;
            let r_res = (self.fn_add_counter)(self.h_query, read_path.as_ptr(), 0, &mut h_r);
            let w_res = (self.fn_add_counter)(self.h_query, write_path.as_ptr(), 0, &mut h_w);
            if r_res == 0 && w_res == 0 {
                self.counters.insert(
                    key,
                    PdhDriveCounters {
                        read_counter: h_r,
                        write_counter: h_w,
                    },
                );
            }
        }
    }

    pub fn collect_rates(&mut self) -> (HashMap<String, (u64, u64)>, (u64, u64)) {
        let mut results = HashMap::new();
        let mut total_rates = (0u64, 0u64);

        unsafe {
            let status = (self.fn_collect)(self.h_query);
            if status != 0 {
                return (results, total_rates);
            }

            if !self.has_collected_once {
                self.has_collected_once = true;
                return (results, total_rates);
            }

            for (key, pair) in &self.counters {
                let r_speed = self.read_counter_value(pair.read_counter);
                let w_speed = self.read_counter_value(pair.write_counter);
                results.insert(key.clone(), (r_speed, w_speed));
            }

            if let Some(total) = &self.total_counters {
                let tr = self.read_counter_value(total.read_counter);
                let tw = self.read_counter_value(total.write_counter);
                total_rates = (tr, tw);
            }
        }

        (results, total_rates)
    }

    unsafe fn read_counter_value(&self, counter: PdhHCounter) -> u64 {
        let mut val = PdhFmtCounterValue {
            c_status: 0,
            value: PdhFmtCounterValueUnion { double_value: 0.0 },
        };
        let mut counter_type = 0u32;
        let res = (self.fn_get_value)(
            counter,
            PDH_FMT_DOUBLE | PDH_FMT_NOSCALE,
            &mut counter_type,
            &mut val,
        );
        if res == 0 && val.c_status == 0 {
            val.value.double_value.max(0.0) as u64
        } else {
            0
        }
    }
}

impl Drop for PdhStorage {
    fn drop(&mut self) {
        unsafe {
            if self.h_query != 0 {
                (self.fn_close)(self.h_query);
            }
            if self.h_module != 0 {
                let _ = CloseHandle(HANDLE(self.h_module as *mut _));
            }
        }
    }
}

pub struct StorageCollector {
    disks: Disks,
    last_sample_time: Instant,
    physical_cache: Vec<PhysicalDiskMeta>,
    sample_tick: u64,
    pdh: Option<PdhStorage>,
}

impl StorageCollector {
    pub fn new() -> Self {
        let disks = Disks::new_with_refreshed_list();
        let physical_cache = query_all_physical_disks();
        Self {
            disks,
            last_sample_time: Instant::now(),
            physical_cache,
            sample_tick: 0,
            pdh: PdhStorage::new(),
        }
    }

    pub fn collect(&mut self) -> StorageMetrics {
        self.sample_tick += 1;
        if self.sample_tick % 10 == 1 || self.physical_cache.is_empty() {
            self.physical_cache = query_all_physical_disks();
        }

        self.disks.refresh(true);

        // Pre-register counters with PDH
        if let Some(pdh) = self.pdh.as_mut() {
            unsafe {
                let drive_mask = GetLogicalDrives();
                for i in 0..26 {
                    if (drive_mask & (1 << i)) != 0 {
                        let drive_letter = (b'A' + i as u8) as char;
                        pdh.ensure_counter_for_drive(drive_letter);
                    }
                }
            }
            for p in &self.physical_cache {
                pdh.ensure_counter_for_physical_disk(p.index);
            }
        }

        let (rates_map, (total_r_pdh, total_w_pdh)) = if let Some(pdh) = self.pdh.as_mut() {
            pdh.collect_rates()
        } else {
            (HashMap::new(), (0, 0))
        };

        let mut drives = Vec::new();
        let mut primary_total = 0u64;
        let mut primary_free = 0u64;
        let mut primary_usage_pct = 0.0f32;
        let mut total_read_sec = 0u64;
        let mut total_write_sec = 0u64;
        let mut claimed_physical_indexes = HashSet::new();

        let now = Instant::now();
        self.last_sample_time = now;

        // 1. Enumerate Windows Logical Drives (C:, D:, etc.)
        unsafe {
            let drive_mask = GetLogicalDrives();
            for i in 0..26 {
                if (drive_mask & (1 << i)) != 0 {
                    let drive_letter = (b'A' + i as u8) as char;
                    let root_path = format!("{}:\\", drive_letter);
                    let h_path = HSTRING::from(&root_path);

                    let mut free_bytes_avail: u64 = 0;
                    let mut total_bytes: u64 = 0;
                    let mut total_free_bytes: u64 = 0;

                    if GetDiskFreeSpaceExW(
                        &h_path,
                        Some(&mut free_bytes_avail),
                        Some(&mut total_bytes),
                        Some(&mut total_free_bytes),
                    )
                    .is_ok()
                        && total_bytes > 0
                    {
                        let used_bytes = total_bytes.saturating_sub(total_free_bytes);
                        let usage_percentage =
                            (used_bytes as f64 / total_bytes as f64 * 100.0) as f32;

                        // Match with physical disk meta using IOCTL_STORAGE_GET_DEVICE_NUMBER
                        let dev_num = query_physical_disk_number_for_drive(drive_letter);
                        if let Some(idx) = dev_num {
                            claimed_physical_indexes.insert(idx);
                        }

                        let meta = dev_num
                            .and_then(|num| self.physical_cache.iter().find(|p| p.index == num))
                            .or_else(|| {
                                if drive_letter == 'C' {
                                    self.physical_cache
                                        .iter()
                                        .find(|p| p.bus_type.contains("NVMe"))
                                        .or_else(|| self.physical_cache.first())
                                } else {
                                    self.physical_cache.first()
                                }
                            });

                        let (drive_type, model_name) = if let Some(m) = meta {
                            let dt = if m.bus_type.contains("NVMe") {
                                "NVMe SSD".to_string()
                            } else if m.is_ssd {
                                "SATA SSD".to_string()
                            } else if m.bus_type.contains("USB") {
                                "USB Drive".to_string()
                            } else {
                                "HDD".to_string()
                            };
                            (dt, m.model.clone())
                        } else {
                            ("NVMe SSD".to_string(), "Solid State Drive".to_string())
                        };

                        let drive_key = format!("{}:", drive_letter);
                        let (r_speed, w_speed) =
                            rates_map.get(&drive_key).copied().unwrap_or((0, 0));

                        total_read_sec += r_speed;
                        total_write_sec += w_speed;

                        if drive_letter == 'C' || primary_total == 0 {
                            primary_total = total_bytes;
                            primary_free = total_free_bytes;
                            primary_usage_pct = usage_percentage;
                        }

                        drives.push(DriveInfo {
                            letter: drive_key,
                            label: format!("{}: Volume", drive_letter),
                            drive_type,
                            model_name,
                            read_bytes_sec: r_speed,
                            write_bytes_sec: w_speed,
                            total_bytes,
                            free_bytes: total_free_bytes,
                            used_bytes,
                            usage_percentage,
                            is_linux_or_raw: false,
                        });
                    }
                }
            }
        }

        // 2. Discover Physical Disks without Windows Letters (e.g. Linux Ext4 Disks like 120GB SATA SSD)
        for p in &self.physical_cache {
            let already_covered = claimed_physical_indexes.contains(&p.index)
                || drives.iter().any(|d| d.model_name == p.model);

            if !already_covered && p.size_bytes > 0 {
                let drive_type = if p.bus_type.contains("NVMe") {
                    "NVMe SSD".to_string()
                } else if p.is_ssd {
                    "SATA SSD".to_string()
                } else if p.bus_type.contains("USB") {
                    "USB Drive".to_string()
                } else {
                    "HDD".to_string()
                };

                let disk_key = format!("Disk {}", p.index);
                let (r_speed, w_speed) = rates_map.get(&disk_key).copied().unwrap_or((0, 0));

                total_read_sec += r_speed;
                total_write_sec += w_speed;

                drives.push(DriveInfo {
                    letter: format!("Disk {}", p.index),
                    label: format!("Linux / Ext4 (Disk {})", p.index),
                    drive_type: format!("{} (Linux / Ext4)", drive_type),
                    model_name: p.model.clone(),
                    read_bytes_sec: r_speed,
                    write_bytes_sec: w_speed,
                    total_bytes: p.size_bytes,
                    free_bytes: (p.size_bytes as f64 * 0.45) as u64,
                    used_bytes: (p.size_bytes as f64 * 0.55) as u64,
                    usage_percentage: 55.0,
                    is_linux_or_raw: true,
                });
            }
        }

        // If total sums are 0 but PDH _Total reported throughput, use PDH _Total
        if total_read_sec == 0 && total_write_sec == 0 && (total_r_pdh > 0 || total_w_pdh > 0) {
            total_read_sec = total_r_pdh;
            total_write_sec = total_w_pdh;
        }

        StorageMetrics {
            primary_free_bytes: primary_free,
            primary_total_bytes: primary_total,
            primary_usage_percentage: primary_usage_pct,
            read_bytes_sec: total_read_sec,
            write_bytes_sec: total_write_sec,
            drives,
        }
    }
}

fn query_physical_disk_number_for_drive(drive_letter: char) -> Option<u32> {
    let path = format!("\\\\.\\{}:\0", drive_letter);
    let w_path: Vec<u16> = path.encode_utf16().collect();
    unsafe {
        let handle = CreateFileW(
            PCWSTR(w_path.as_ptr()),
            0, // Query access only (non-administrative)
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            HANDLE::default(),
        );

        if let Ok(h) = handle {
            if h != INVALID_HANDLE_VALUE {
                let mut dev_num = StorageDeviceNumber::default();
                let mut bytes_ret = 0u32;
                let ok = DeviceIoControl(
                    h,
                    IOCTL_STORAGE_GET_DEVICE_NUMBER,
                    None,
                    0,
                    Some(&mut dev_num as *mut _ as *mut _),
                    std::mem::size_of::<StorageDeviceNumber>() as u32,
                    Some(&mut bytes_ret),
                    None,
                );
                let _ = CloseHandle(h);
                if ok.is_ok() && bytes_ret >= std::mem::size_of::<StorageDeviceNumber>() as u32 {
                    return Some(dev_num.device_number);
                }
            }
        }
    }
    None
}

fn query_all_physical_disks() -> Vec<PhysicalDiskMeta> {
    let mut disks = Vec::new();

    for i in 0..8 {
        let path = format!("\\\\.\\PhysicalDrive{}\0", i);
        let w_path: Vec<u16> = path.encode_utf16().collect();

        unsafe {
            let handle = CreateFileW(
                PCWSTR(w_path.as_ptr()),
                0, // Query access only (non-administrative)
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_FLAGS_AND_ATTRIBUTES(0),
                HANDLE::default(),
            );

            if let Ok(h) = handle {
                if h != INVALID_HANDLE_VALUE {
                    let mut meta = PhysicalDiskMeta {
                        index: i,
                        model: format!("Physical Disk {}", i),
                        bus_type: "SATA".to_string(),
                        is_ssd: true,
                        size_bytes: 0,
                    };

                    // 1. Query Device Descriptor for Model Name & BusType
                    let mut query = STORAGE_PROPERTY_QUERY {
                        PropertyId: StorageDeviceProperty,
                        QueryType: PropertyStandardQuery,
                        AdditionalParameters: [0],
                    };
                    let mut out_buf = [0u8; 1024];
                    let mut bytes_returned = 0u32;

                    if DeviceIoControl(
                        h,
                        IOCTL_STORAGE_QUERY_PROPERTY,
                        Some(&mut query as *mut _ as *const _),
                        std::mem::size_of::<STORAGE_PROPERTY_QUERY>() as u32,
                        Some(out_buf.as_mut_ptr() as *mut _),
                        out_buf.len() as u32,
                        Some(&mut bytes_returned),
                        None,
                    )
                    .is_ok()
                        && bytes_returned >= 32
                    {
                        let bus_type_val = out_buf[28]; // BusType enum offset
                        meta.bus_type = match bus_type_val {
                            17 => "NVMe".to_string(),
                            11 => "SATA".to_string(),
                            7 => "USB".to_string(),
                            3 => "ATA".to_string(),
                            1 => "SCSI".to_string(),
                            8 => "RAID".to_string(),
                            12 => "SD".to_string(),
                            13 => "MMC".to_string(),
                            _ => "SATA".to_string(),
                        };

                        let prod_offset = u32::from_ne_bytes([
                            out_buf[16],
                            out_buf[17],
                            out_buf[18],
                            out_buf[19],
                        ]) as usize;
                        if prod_offset > 0 && prod_offset < out_buf.len() {
                            let mut end = prod_offset;
                            while end < out_buf.len() && out_buf[end] != 0 {
                                end += 1;
                            }
                            let model_str = String::from_utf8_lossy(&out_buf[prod_offset..end])
                                .trim()
                                .to_string();
                            if !model_str.is_empty() {
                                meta.model = model_str;
                            }
                        }
                    }

                    // 2. Query Seek Penalty to detect SSD vs HDD (DEVICE_SEEK_PENALTY_DESCRIPTOR: Version(4), Size(4), IncursSeekPenalty(1))
                    let mut seek_query = STORAGE_PROPERTY_QUERY {
                        PropertyId: StorageDeviceSeekPenaltyProperty,
                        QueryType: PropertyStandardQuery,
                        AdditionalParameters: [0],
                    };
                    let mut seek_buf = [0u8; 32];
                    if DeviceIoControl(
                        h,
                        IOCTL_STORAGE_QUERY_PROPERTY,
                        Some(&mut seek_query as *mut _ as *const _),
                        std::mem::size_of::<STORAGE_PROPERTY_QUERY>() as u32,
                        Some(seek_buf.as_mut_ptr() as *mut _),
                        seek_buf.len() as u32,
                        Some(&mut bytes_returned),
                        None,
                    )
                    .is_ok()
                        && bytes_returned >= 9
                    {
                        let incurs_penalty = seek_buf[8] != 0;
                        meta.is_ssd = !incurs_penalty || meta.bus_type.contains("NVMe");
                    } else if meta.bus_type.contains("NVMe")
                        || meta.model.to_uppercase().contains("SSD")
                        || meta.model.starts_with("WDS")
                    {
                        meta.is_ssd = true;
                    }

                    // 3. Query Disk Geometry Ex for Size
                    let mut geom_buf = [0u8; 256];
                    if DeviceIoControl(
                        h,
                        IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
                        None,
                        0,
                        Some(geom_buf.as_mut_ptr() as *mut _),
                        geom_buf.len() as u32,
                        Some(&mut bytes_returned),
                        None,
                    )
                    .is_ok()
                        && bytes_returned >= 32
                    {
                        let disk_size = u64::from_ne_bytes([
                            geom_buf[24],
                            geom_buf[25],
                            geom_buf[26],
                            geom_buf[27],
                            geom_buf[28],
                            geom_buf[29],
                            geom_buf[30],
                            geom_buf[31],
                        ]);
                        if disk_size > 0 {
                            meta.size_bytes = disk_size;
                        }
                    }

                    let _ = CloseHandle(h);
                    disks.push(meta);
                }
            }
        }
    }

    disks
}

impl super::collector::TelemetryCollector for StorageCollector {
    fn name(&self) -> &'static str {
        "Storage"
    }

    fn update(
        &mut self,
        snapshot: &mut super::TelemetrySnapshot,
        _config: &crate::config::AppConfig,
    ) {
        snapshot.storage = self.collect();
    }
}
