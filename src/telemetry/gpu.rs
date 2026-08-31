#![allow(unused_imports, dead_code, unused_must_use)]

use windows::core::{w, Interface, PCWSTR, PWSTR};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIAdapter1, IDXGIAdapter3, IDXGIFactory1,
    DXGI_ADAPTER_FLAG_SOFTWARE, DXGI_MEMORY_SEGMENT_GROUP_LOCAL,
    DXGI_MEMORY_SEGMENT_GROUP_NON_LOCAL, DXGI_QUERY_VIDEO_MEMORY_INFO,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE,
    KEY_READ, REG_SZ, REG_VALUE_TYPE,
};

#[derive(Debug, Clone, Default)]
pub struct GpuInfo {
    pub name: String,
    pub vendor: String,
    pub gpu_type: String, // "Discrete GPU" | "Integrated GPU"
    pub vram_used_bytes: u64,
    pub vram_total_bytes: u64,
    pub vram_usage_percentage: f32,
    pub shared_used_bytes: u64,
    pub shared_total_bytes: u64,
    pub shared_usage_percentage: f32,
    pub dedicated_vram_bytes: u64,
    pub shared_vram_bytes: u64,
    pub vendor_id: u32,
    pub device_id: u32,
    pub is_active: bool,
}

#[derive(Debug, Clone, Default)]
pub struct GpuMetrics {
    pub gpus: Vec<GpuInfo>,
    pub primary_gpu_name: String,
    pub primary_vendor: String,
    pub primary_type: String,
    pub primary_vram_used_gb: f32,
    pub primary_vram_total_gb: f32,
    pub primary_vram_usage_percentage: f32,
    pub primary_shared_used_gb: f32,
    pub primary_shared_total_gb: f32,
    pub primary_shared_usage_percentage: f32,
}

pub struct GpuCollector {
    factory: Option<IDXGIFactory1>,
}

impl GpuCollector {
    pub fn new() -> Self {
        let factory = unsafe { CreateDXGIFactory1::<IDXGIFactory1>().ok() };
        Self { factory }
    }

    pub fn collect(&mut self) -> GpuMetrics {
        let mut gpus = Vec::new();

        // 1. Enumerate active DXGI adapters
        if let Some(factory) = &self.factory {
            unsafe {
                let mut index = 0;
                while let Ok(adapter) = factory.EnumAdapters1(index) {
                    if let Ok(desc) = adapter.GetDesc1() {
                        let is_software = (desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) != 0;
                        let name_raw = String::from_utf16_lossy(&desc.Description);
                        let clean_name = name_raw.trim_matches(char::from(0)).trim().to_string();

                        let dedicated_video = desc.DedicatedVideoMemory as u64;
                        let shared_system = desc.SharedSystemMemory as u64;

                        let mut vram_used: u64 = 0;
                        let mut vram_total: u64 = dedicated_video;
                        let mut shared_used: u64 = 0;
                        let mut shared_total: u64 = shared_system;

                        if let Ok(adapter3) = adapter.cast::<IDXGIAdapter3>() {
                            let mut local_info = DXGI_QUERY_VIDEO_MEMORY_INFO::default();
                            if adapter3.QueryVideoMemoryInfo(0, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, &mut local_info).is_ok() {
                                vram_used = local_info.CurrentUsage;
                                if local_info.Budget > 0 {
                                    vram_total = local_info.Budget;
                                }
                            }

                            let mut non_local_info = DXGI_QUERY_VIDEO_MEMORY_INFO::default();
                            if adapter3.QueryVideoMemoryInfo(0, DXGI_MEMORY_SEGMENT_GROUP_NON_LOCAL, &mut non_local_info).is_ok() {
                                shared_used = non_local_info.CurrentUsage;
                                if non_local_info.Budget > 0 {
                                    shared_total = non_local_info.Budget;
                                }
                            }
                        }

                        let vram_usage_pct = if vram_total > 0 {
                            (vram_used as f32 / vram_total as f32) * 100.0
                        } else {
                            0.0
                        };

                        let shared_usage_pct = if shared_total > 0 {
                            (shared_used as f32 / shared_total as f32) * 100.0
                        } else {
                            0.0
                        };

                        let vendor = match desc.VendorId {
                            0x10DE => "NVIDIA",
                            0x1002 | 0x1022 => "AMD",
                            0x8086 => "Intel",
                            0x1414 => "Microsoft",
                            _ => "Graphics",
                        }.to_string();

                        let gpu_type = if dedicated_video > 512 * 1024 * 1024 || vendor == "NVIDIA" {
                            "Discrete GPU".to_string()
                        } else {
                            "Integrated GPU".to_string()
                        };

                        if !is_software || gpus.is_empty() {
                            gpus.push(GpuInfo {
                                name: clean_name,
                                vendor,
                                gpu_type,
                                vram_used_bytes: vram_used,
                                vram_total_bytes: vram_total,
                                vram_usage_percentage: vram_usage_pct.clamp(0.0, 100.0),
                                shared_used_bytes: shared_used,
                                shared_total_bytes: shared_total,
                                shared_usage_percentage: shared_usage_pct.clamp(0.0, 100.0),
                                dedicated_vram_bytes: dedicated_video,
                                shared_vram_bytes: shared_system,
                                vendor_id: desc.VendorId,
                                device_id: desc.DeviceId,
                                is_active: true,
                            });
                        }
                    }
                    index += 1;
                }
            }
        }

        // 2. Discover all registered Display Adapters from Windows Registry (covers disabled dGPUs, D3Cold sleep, etc.)
        let reg_gpus = discover_registry_gpus();
        for reg_gpu in reg_gpus {
            let already_present = gpus.iter().any(|g| {
                let g_low = g.name.to_lowercase();
                let reg_low = reg_gpu.name.to_lowercase();
                g_low == reg_low || (g_low.contains("radeon") && reg_low.contains("radeon")) || (g_low.contains("nvidia") && reg_low.contains("nvidia"))
            });

            if !already_present {
                gpus.push(reg_gpu);
            }
        }

        // Prefer discrete GPU or first active adapter
        let primary = gpus.iter()
            .find(|g| g.is_active && (g.dedicated_vram_bytes > 512 * 1024 * 1024 || g.gpu_type.contains("Discrete")))
            .or_else(|| gpus.iter().find(|g| g.is_active))
            .or_else(|| gpus.first())
            .cloned()
            .unwrap_or_default();

        let used_gb = primary.vram_used_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
        let total_gb = primary.vram_total_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
        let shared_used_gb = primary.shared_used_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
        let shared_total_gb = primary.shared_total_bytes as f32 / (1024.0 * 1024.0 * 1024.0);

        GpuMetrics {
            primary_gpu_name: if primary.name.is_empty() { "Graphics Adapter".to_string() } else { primary.name },
            primary_vendor: primary.vendor,
            primary_type: primary.gpu_type,
            primary_vram_used_gb: used_gb,
            primary_vram_total_gb: total_gb.max(0.1),
            primary_vram_usage_percentage: primary.vram_usage_percentage,
            primary_shared_used_gb: shared_used_gb,
            primary_shared_total_gb: shared_total_gb.max(0.1),
            primary_shared_usage_percentage: primary.shared_usage_percentage,
            gpus,
        }
    }
}

pub fn discover_registry_gpus() -> Vec<GpuInfo> {
    let mut results = Vec::new();
    unsafe {
        let class_path = w!("SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}");
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, class_path, 0, KEY_READ, &mut hkey).is_ok() {
            let mut index = 0;
            loop {
                let mut subkey_name = [0u16; 256];
                let mut name_len = subkey_name.len() as u32;
                if RegEnumKeyExW(
                    hkey,
                    index,
                    PWSTR(subkey_name.as_mut_ptr()),
                    &mut name_len,
                    None,
                    PWSTR::null(),
                    None,
                    None,
                ).is_err() {
                    break;
                }
                index += 1;

                let sub_str = String::from_utf16_lossy(&subkey_name[..name_len as usize]);
                if sub_str.starts_with("0") {
                    let subkey_wstr = format!("{}\0", sub_str).encode_utf16().collect::<Vec<u16>>();
                    let mut sub_hkey = HKEY::default();
                    if RegOpenKeyExW(hkey, PCWSTR(subkey_wstr.as_ptr()), 0, KEY_READ, &mut sub_hkey).is_ok() {
                        let mut desc_buf = [0u8; 512];
                        let mut desc_len = desc_buf.len() as u32;
                        let mut val_type = REG_VALUE_TYPE::default();

                        let val_name = w!("DriverDesc");
                        if RegQueryValueExW(
                            sub_hkey,
                            val_name,
                            None,
                            Some(&mut val_type),
                            Some(desc_buf.as_mut_ptr()),
                            Some(&mut desc_len),
                        ).is_ok() && desc_len > 0 {
                            let u16_slice = std::slice::from_raw_parts(
                                desc_buf.as_ptr() as *const u16,
                                (desc_len as usize / 2).saturating_sub(1),
                            );
                            let driver_desc = String::from_utf16_lossy(u16_slice).trim().to_string();

                            if !driver_desc.is_empty() && !driver_desc.contains("Basic Render") {
                                let mut prov_buf = [0u8; 256];
                                let mut prov_len = prov_buf.len() as u32;
                                let prov_name = w!("ProviderName");
                                let mut vendor = "Graphics".to_string();
                                if RegQueryValueExW(
                                    sub_hkey,
                                    prov_name,
                                    None,
                                    None,
                                    Some(prov_buf.as_mut_ptr()),
                                    Some(&mut prov_len),
                                ).is_ok() && prov_len > 0 {
                                    let prov_u16 = std::slice::from_raw_parts(
                                        prov_buf.as_ptr() as *const u16,
                                        (prov_len as usize / 2).saturating_sub(1),
                                    );
                                    let prov_str = String::from_utf16_lossy(prov_u16).to_lowercase();
                                    if prov_str.contains("nvidia") {
                                        vendor = "NVIDIA".to_string();
                                    } else if prov_str.contains("amd") || prov_str.contains("advanced micro") {
                                        vendor = "AMD".to_string();
                                    } else if prov_str.contains("intel") {
                                        vendor = "Intel".to_string();
                                    }
                                }

                                let is_discrete = driver_desc.to_lowercase().contains("rtx")
                                    || driver_desc.to_lowercase().contains("gtx")
                                    || driver_desc.to_lowercase().contains("geforce")
                                    || driver_desc.to_lowercase().contains("radeon rx")
                                    || driver_desc.to_lowercase().contains("discrete")
                                    || vendor == "NVIDIA";

                                results.push(GpuInfo {
                                    name: driver_desc,
                                    vendor,
                                    gpu_type: if is_discrete { "Discrete GPU".to_string() } else { "Integrated GPU".to_string() },
                                    is_active: false,
                                    ..Default::default()
                                });
                            }
                        }
                        let _ = RegCloseKey(sub_hkey);
                    }
                }
            }
            let _ = RegCloseKey(hkey);
        }
    }
    results
}

impl super::collector::TelemetryCollector for GpuCollector {
    fn name(&self) -> &'static str {
        "GPU"
    }

    fn update(&mut self, snapshot: &mut super::TelemetrySnapshot, _config: &crate::config::AppConfig) {
        snapshot.gpu = self.collect();
    }
}
