use std::time::Instant;
use windows::Win32::System::ProcessStatus::{GetPerformanceInfo, PERFORMANCE_INFORMATION};
use windows::Win32::System::SystemInformation::{
    GetSystemFirmwareTable, GetTickCount64, GlobalMemoryStatusEx, MEMORYSTATUSEX,
};

use super::pdh::{PdhHCounter, PdhHelper};

#[derive(Debug, Clone, Default)]
pub struct RamMetrics {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub cached_bytes: u64,
    pub usage_percentage: f32,

    // Virtual Memory & Commit Metrics
    pub committed_bytes: u64,
    pub commit_limit_bytes: u64,
    pub page_faults_per_sec: u64,
    pub page_file_usage_pct: f32,

    // System Overview
    pub process_count: u32,
    pub thread_count: u32,
    pub handle_count: u32,
    pub uptime_formatted: String,

    // Advanced Metrics (gated by config.adv_ram / adv_virtual_memory)
    pub hardware_reserved_bytes: u64,
    pub modified_bytes: u64,
    pub standby_bytes: u64,
    pub nonpaged_pool_bytes: u64,
    pub paged_pool_bytes: u64,
    pub system_cache_bytes: u64,
    pub ram_speed_mhz: Option<u32>,
    pub ram_type: String,
    pub ram_slots_used: u32,
    pub ram_slots_total: u32,
}

pub struct RamCollector {
    last_sample_time: Instant,
    pdh: Option<PdhHelper>,
    h_modified: PdhHCounter,
    h_standby_norm: PdhHCounter,
    h_standby_res: PdhHCounter,
    h_page_faults: PdhHCounter,
    ram_speed_mhz: Option<u32>,
    ram_type: String,
    ram_slots_used: u32,
    ram_slots_total: u32,
    hw_installed_bytes: u64,
}

impl RamCollector {
    pub fn new() -> Self {
        let (ram_type, ram_speed_mhz, ram_slots_used, ram_slots_total, hw_installed_bytes) =
            query_smbios_dram_specs();

        let mut pdh = PdhHelper::new();
        let mut h_modified = 0;
        let mut h_standby_norm = 0;
        let mut h_standby_res = 0;
        let mut h_page_faults = 0;

        if let Some(p) = pdh.as_mut() {
            h_modified = p.add_counter("\\Memory\\Modified Page List Bytes");
            h_standby_norm = p.add_counter("\\Memory\\Standby Cache Normal Priority Bytes");
            h_standby_res = p.add_counter("\\Memory\\Standby Cache Reserve Bytes");
            h_page_faults = p.add_counter("\\Memory\\Page Faults/sec");
        }

        Self {
            last_sample_time: Instant::now(),
            pdh,
            h_modified,
            h_standby_norm,
            h_standby_res,
            h_page_faults,
            ram_speed_mhz,
            ram_type,
            ram_slots_used,
            ram_slots_total,
            hw_installed_bytes,
        }
    }

    pub fn collect(&mut self) -> RamMetrics {
        unsafe {
            let mut mem_status = MEMORYSTATUSEX {
                dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
                ..Default::default()
            };

            let mut perf_info = PERFORMANCE_INFORMATION {
                cb: std::mem::size_of::<PERFORMANCE_INFORMATION>() as u32,
                ..Default::default()
            };

            let _ = GlobalMemoryStatusEx(&mut mem_status);
            let perf_ok = GetPerformanceInfo(&mut perf_info, perf_info.cb).is_ok();

            let total_bytes = mem_status.ullTotalPhys;
            let free_bytes = mem_status.ullAvailPhys;
            let used_bytes = total_bytes.saturating_sub(free_bytes);
            let usage_percentage = mem_status.dwMemoryLoad as f32;

            let page_size = if perf_ok && perf_info.PageSize > 0 {
                perf_info.PageSize as u64
            } else {
                4096
            };

            let committed_bytes = if perf_ok {
                perf_info.CommitTotal as u64 * page_size
            } else {
                mem_status
                    .ullTotalPageFile
                    .saturating_sub(mem_status.ullAvailPageFile)
            };

            let commit_limit_bytes = if perf_ok {
                perf_info.CommitLimit as u64 * page_size
            } else {
                mem_status.ullTotalPageFile
            };

            let paged_pool_bytes = if perf_ok {
                perf_info.KernelPaged as u64 * page_size
            } else {
                0
            };

            let nonpaged_pool_bytes = if perf_ok {
                perf_info.KernelNonpaged as u64 * page_size
            } else {
                0
            };

            let system_cache_bytes = if perf_ok {
                perf_info.SystemCache as u64 * page_size
            } else {
                0
            };

            // PDH sampling for fine-grained page list metrics
            let mut modified_bytes = 0u64;
            let mut standby_bytes = 0u64;
            let mut pdh_faults = 0u64;

            if let Some(p) = self.pdh.as_mut() {
                if p.collect() {
                    modified_bytes = p.read_u64(self.h_modified);
                    let norm = p.read_u64(self.h_standby_norm);
                    let res = p.read_u64(self.h_standby_res);
                    standby_bytes = norm + res;
                    pdh_faults = p.read_u64(self.h_page_faults);
                }
            }

            let cached_bytes = if standby_bytes + modified_bytes > 0 {
                (standby_bytes + modified_bytes).min(total_bytes)
            } else if system_cache_bytes > 0 {
                system_cache_bytes.min(total_bytes)
            } else {
                total_bytes.saturating_sub(used_bytes)
            };

            let now = Instant::now();
            self.last_sample_time = now;

            let page_faults_per_sec = pdh_faults;

            // Page file usage percentage
            let page_file_total = mem_status
                .ullTotalPageFile
                .saturating_sub(total_bytes)
                .max(1);
            let page_file_free = mem_status.ullAvailPageFile.saturating_sub(free_bytes);
            let page_file_used = page_file_total.saturating_sub(page_file_free);
            let page_file_usage_pct =
                (page_file_used as f64 / page_file_total as f64 * 100.0).clamp(0.0, 100.0) as f32;

            let process_count = if perf_ok { perf_info.ProcessCount } else { 0 };
            let thread_count = if perf_ok { perf_info.ThreadCount } else { 0 };
            let handle_count = if perf_ok { perf_info.HandleCount } else { 0 };

            let uptime_ms = GetTickCount64();
            let total_secs = uptime_ms / 1000;
            let days = total_secs / 86400;
            let hours = (total_secs % 86400) / 3600;
            let mins = (total_secs % 3600) / 60;
            let secs = total_secs % 60;
            let uptime_formatted = if days > 0 {
                format!("{}d {}h {}m", days, hours, mins)
            } else {
                format!("{}h {}m {}s", hours, mins, secs)
            };

            let hardware_reserved_bytes = self.hw_installed_bytes.saturating_sub(total_bytes);

            RamMetrics {
                total_bytes,
                used_bytes,
                free_bytes,
                cached_bytes,
                usage_percentage,
                committed_bytes,
                commit_limit_bytes,
                page_faults_per_sec,
                page_file_usage_pct,
                process_count,
                thread_count,
                handle_count,
                uptime_formatted,
                hardware_reserved_bytes,
                modified_bytes,
                standby_bytes,
                nonpaged_pool_bytes,
                paged_pool_bytes,
                system_cache_bytes,
                ram_speed_mhz: self.ram_speed_mhz,
                ram_type: self.ram_type.clone(),
                ram_slots_used: self.ram_slots_used,
                ram_slots_total: self.ram_slots_total,
            }
        }
    }
}

fn query_smbios_dram_specs() -> (String, Option<u32>, u32, u32, u64) {
    let mut ram_type = "DDR4".to_string();
    let mut ram_speed: Option<u32> = None;
    let mut slots_total = 0u32;
    let mut slots_used = 0u32;
    let mut total_installed_bytes = 0u64;

    unsafe {
        // RSMB signature = 0x52534D42 ("RSMB" in ASCII little-endian)
        let rsmb_provider =
            windows::Win32::System::SystemInformation::FIRMWARE_TABLE_PROVIDER(0x52534D42);
        let buf_size = GetSystemFirmwareTable(rsmb_provider, 0, None);
        if buf_size > 0 {
            let mut buffer = vec![0u8; buf_size as usize];
            if GetSystemFirmwareTable(rsmb_provider, 0, Some(&mut buffer)) > 0 {
                // Parse Raw SMBIOS Table
                // Header is 8 bytes: Used20CallingMethod(1), Major(1), Minor(1), DmiRev(1), Length(4)
                if buffer.len() > 8 {
                    let mut offset = 8;
                    while offset + 4 <= buffer.len() {
                        let struct_type = buffer[offset];
                        let struct_len = buffer[offset + 1] as usize;
                        if struct_len < 4 || offset + struct_len > buffer.len() {
                            break;
                        }

                        // Type 17: Memory Device
                        if struct_type == 17 && struct_len >= 0x14 {
                            slots_total += 1;
                            let size_val =
                                u16::from_le_bytes([buffer[offset + 0x0C], buffer[offset + 0x0D]]);
                            if size_val > 0 && size_val != 0xFFFF {
                                slots_used += 1;
                                let is_kb = (size_val & 0x8000) != 0;
                                let size_num = (size_val & 0x7FFF) as u64;
                                let dev_bytes = if is_kb {
                                    size_num * 1024
                                } else {
                                    size_num * 1024 * 1024
                                };
                                total_installed_bytes += dev_bytes;

                                if struct_len >= 0x14 {
                                    let mem_type_byte = buffer[offset + 0x12];
                                    let detected_type = match mem_type_byte {
                                        0x18 => "DDR3",
                                        0x1A => "DDR4",
                                        0x1E => "LPDDR4",
                                        0x22 => "DDR5",
                                        0x23 => "LPDDR5",
                                        0x1F => "LPDDR3",
                                        _ => "DDR4",
                                    };
                                    ram_type = detected_type.to_string();
                                }

                                if struct_len >= 0x17 {
                                    let speed_val = u16::from_le_bytes([
                                        buffer[offset + 0x15],
                                        buffer[offset + 0x16],
                                    ]);
                                    if speed_val > 0 && speed_val != 0xFFFF {
                                        ram_speed = Some(speed_val as u32);
                                    }
                                }
                            }
                        }

                        // Skip to unformatted string section (double null-terminated)
                        offset += struct_len;
                        while offset + 1 < buffer.len() {
                            if buffer[offset] == 0 && buffer[offset + 1] == 0 {
                                offset += 2;
                                break;
                            }
                            offset += 1;
                        }
                    }
                }
            }
        }
    }

    if slots_total == 0 {
        slots_total = 2;
        slots_used = 1;
    }

    (
        ram_type,
        ram_speed,
        slots_used,
        slots_total,
        total_installed_bytes,
    )
}

impl super::collector::TelemetryCollector for RamCollector {
    fn name(&self) -> &'static str {
        "RAM & System"
    }

    fn update(
        &mut self,
        snapshot: &mut super::TelemetrySnapshot,
        _config: &crate::config::AppConfig,
    ) {
        snapshot.ram = self.collect();
    }
}
