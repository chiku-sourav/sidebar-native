#![allow(unused_imports, dead_code, unused_must_use)]

use windows::core::{w, Interface, PCWSTR, PWSTR};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIAdapter1, IDXGIAdapter3, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE,
    DXGI_MEMORY_SEGMENT_GROUP_LOCAL, DXGI_MEMORY_SEGMENT_GROUP_NON_LOCAL,
    DXGI_QUERY_VIDEO_MEMORY_INFO,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE,
    KEY_READ, REG_SZ, REG_VALUE_TYPE,
};

use super::pdh::{PdhHCounter, PdhHelper};

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
    pub luid: (u32, u32),
    pub is_active: bool,

    // Advanced Metrics (gated by config.adv_gpu)
    pub gpu_usage_pct: f32,
    pub copy_engine_pct: f32,
    pub video_encode_pct: f32,
    pub video_decode_pct: f32,
    pub gpu_clock_mhz: Option<u64>,
    pub driver_version: String,
    pub pcie_gen: Option<u8>,
    pub pcie_width: Option<u8>,
    pub temperature_c: Option<f32>,
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

    // Primary GPU Advanced
    pub primary_driver_version: String,
    pub primary_gpu_usage_pct: f32,
    pub primary_copy_engine_pct: f32,
    pub primary_video_encode_pct: f32,
    pub primary_video_decode_pct: f32,
    pub primary_gpu_clock_mhz: Option<u64>,
    pub primary_pcie_gen: Option<u8>,
    pub primary_pcie_width: Option<u8>,
    pub primary_temperature_c: Option<f32>,
}

pub struct GpuCollector {
    factory: Option<IDXGIFactory1>,
    pdh: Option<PdhHelper>,
    h_gpu_engine: PdhHCounter,
    h_gpu_dedicated: PdhHCounter,
    h_gpu_shared: PdhHCounter,
    nvml: Option<super::nvml::NvmlHelper>,
    registry_gpus_cached: Vec<GpuInfo>,
    cache_tick: u64,
}

impl GpuCollector {
    pub fn new() -> Self {
        let factory = unsafe { CreateDXGIFactory1::<IDXGIFactory1>().ok() };
        let mut pdh = PdhHelper::new();
        let mut h_gpu_engine = 0;
        let mut h_gpu_dedicated = 0;
        let mut h_gpu_shared = 0;
        if let Some(p) = pdh.as_mut() {
            h_gpu_engine = p.add_counter("\\GPU Engine(*)\\Utilization Percentage");
            h_gpu_dedicated = p.add_counter("\\GPU Adapter Memory(*)\\Dedicated Usage");
            h_gpu_shared = p.add_counter("\\GPU Adapter Memory(*)\\Shared Usage");
        }
        let nvml = super::nvml::NvmlHelper::new();
        let registry_gpus_cached = discover_registry_gpus();

        Self {
            factory,
            pdh,
            h_gpu_engine,
            h_gpu_dedicated,
            h_gpu_shared,
            nvml,
            registry_gpus_cached,
            cache_tick: 0,
        }
    }

    pub fn collect(&mut self) -> GpuMetrics {
        self.cache_tick += 1;
        if self.cache_tick % 30 == 1 || self.registry_gpus_cached.is_empty() {
            self.registry_gpus_cached = discover_registry_gpus();
        }

        // Query NVML for real hardware GPU status (temperature, true PCIe link specs)
        let nv_gpus = self
            .nvml
            .as_ref()
            .map(|n| n.query_gpus())
            .unwrap_or_default();

        // Sample PDH per-adapter GPU engine utilization & memory counters
        let mut engine_by_luid: std::collections::HashMap<String, (f32, f32, f32, f32)> =
            std::collections::HashMap::new();
        let mut memory_by_luid: std::collections::HashMap<String, (u64, u64)> =
            std::collections::HashMap::new();

        if let Some(p) = self.pdh.as_mut() {
            if p.collect() {
                let instances = p.read_array(self.h_gpu_engine);
                for inst in instances {
                    let name_low = inst.name.to_lowercase();
                    if let Some(luid_key) = extract_luid_key(&name_low) {
                        let entry = engine_by_luid
                            .entry(luid_key)
                            .or_insert((0.0, 0.0, 0.0, 0.0));
                        let val = inst.value as f32;
                        if name_low.contains("engtype_3d") {
                            entry.0 += val;
                        } else if name_low.contains("engtype_copy") {
                            entry.1 += val;
                        } else if name_low.contains("videoencode") || name_low.contains("encode") {
                            entry.2 += val;
                        } else if name_low.contains("videodecode") || name_low.contains("decode") {
                            entry.3 += val;
                        }
                    }
                }

                let ded_instances = p.read_array(self.h_gpu_dedicated);
                for inst in ded_instances {
                    let name_low = inst.name.to_lowercase();
                    if let Some(luid_key) = extract_luid_key(&name_low) {
                        let entry = memory_by_luid.entry(luid_key).or_insert((0, 0));
                        entry.0 = entry.0.max(inst.value as u64);
                    }
                }

                let shared_instances = p.read_array(self.h_gpu_shared);
                for inst in shared_instances {
                    let name_low = inst.name.to_lowercase();
                    if let Some(luid_key) = extract_luid_key(&name_low) {
                        let entry = memory_by_luid.entry(luid_key).or_insert((0, 0));
                        entry.1 = entry.1.max(inst.value as u64);
                    }
                }
            }
        }

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

                        let adapter_luid =
                            (desc.AdapterLuid.HighPart as u32, desc.AdapterLuid.LowPart);
                        let luid_key =
                            format!("luid_0x{:08x}_0x{:08x}", adapter_luid.0, adapter_luid.1);

                        // Per-adapter engine utilization matching this GPU's LUID
                        let (eng_3d, eng_copy, eng_enc, eng_dec) = engine_by_luid
                            .get(&luid_key)
                            .copied()
                            .unwrap_or((0.0, 0.0, 0.0, 0.0));

                        let mut vram_used: u64 = 0;
                        let mut shared_used: u64 = 0;

                        if let Some(&(pdh_ded, pdh_shared)) = memory_by_luid.get(&luid_key) {
                            vram_used = pdh_ded;
                            shared_used = pdh_shared;
                        }

                        // Query DXGI memory info for fallback / current usage
                        if let Ok(adapter3) = adapter.cast::<IDXGIAdapter3>() {
                            let mut local_info = DXGI_QUERY_VIDEO_MEMORY_INFO::default();
                            if adapter3
                                .QueryVideoMemoryInfo(
                                    0,
                                    DXGI_MEMORY_SEGMENT_GROUP_LOCAL,
                                    &mut local_info,
                                )
                                .is_ok()
                            {
                                if vram_used == 0 {
                                    vram_used = local_info.CurrentUsage;
                                }
                            }

                            let mut non_local_info = DXGI_QUERY_VIDEO_MEMORY_INFO::default();
                            if adapter3
                                .QueryVideoMemoryInfo(
                                    0,
                                    DXGI_MEMORY_SEGMENT_GROUP_NON_LOCAL,
                                    &mut non_local_info,
                                )
                                .is_ok()
                            {
                                if shared_used == 0 {
                                    shared_used = non_local_info.CurrentUsage;
                                }
                            }
                        }

                        let vram_total = if dedicated_video > 0 {
                            dedicated_video
                        } else {
                            shared_system
                        };
                        let shared_total = shared_system;

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
                        }
                        .to_string();

                        let gpu_type = if dedicated_video > 512 * 1024 * 1024 || vendor == "NVIDIA"
                        {
                            "Discrete GPU".to_string()
                        } else {
                            "Integrated GPU".to_string()
                        };

                        let driver_ver = self
                            .registry_gpus_cached
                            .iter()
                            .find(|r| {
                                r.name.eq_ignore_ascii_case(&clean_name)
                                    || (r.vendor_id == desc.VendorId
                                        && r.device_id == desc.DeviceId)
                            })
                            .map(|r| r.driver_version.clone())
                            .unwrap_or_default();

                        // Query NVML status for NVIDIA GPUs
                        let mut temp_c: Option<f32> = None;
                        let mut pcie_gen: Option<u8> = None;
                        let mut pcie_width: Option<u8> = None;

                        if vendor == "NVIDIA" {
                            if let Some(nv) = nv_gpus
                                .iter()
                                .find(|(n, _, _, _)| {
                                    let n_low = n.to_lowercase();
                                    clean_name.to_lowercase().contains(&n_low)
                                        || n_low.contains(&clean_name.to_lowercase())
                                })
                                .or_else(|| nv_gpus.first())
                            {
                                temp_c = Some(nv.1);
                                pcie_gen = nv.2;
                                pcie_width = nv.3;
                            }
                        }

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
                                luid: adapter_luid,
                                is_active: true,
                                gpu_usage_pct: eng_3d.clamp(0.0, 100.0),
                                copy_engine_pct: eng_copy.clamp(0.0, 100.0),
                                video_encode_pct: eng_enc.clamp(0.0, 100.0),
                                video_decode_pct: eng_dec.clamp(0.0, 100.0),
                                gpu_clock_mhz: None,
                                driver_version: driver_ver,
                                pcie_gen,
                                pcie_width,
                                temperature_c: temp_c,
                            });
                        }
                    }
                    index += 1;
                }
            }
        }

        // 2. Discover all registered Display Adapters from Windows Registry
        for reg_gpu in &self.registry_gpus_cached {
            let already_present = gpus.iter().any(|g| {
                let g_low = g.name.to_lowercase();
                let reg_low = reg_gpu.name.to_lowercase();
                g_low == reg_low
                    || (g_low.contains("radeon") && reg_low.contains("radeon"))
                    || (g_low.contains("nvidia") && reg_low.contains("nvidia"))
            });

            if !already_present {
                gpus.push(reg_gpu.clone());
            }
        }

        // Prefer discrete GPU or first active adapter
        let primary = gpus
            .iter()
            .find(|g| {
                g.is_active
                    && (g.dedicated_vram_bytes > 512 * 1024 * 1024
                        || g.gpu_type.contains("Discrete"))
            })
            .or_else(|| gpus.iter().find(|g| g.is_active))
            .or_else(|| gpus.first())
            .cloned()
            .unwrap_or_default();

        let used_gb = primary.vram_used_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
        let total_gb = primary.vram_total_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
        let shared_used_gb = primary.shared_used_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
        let shared_total_gb = primary.shared_total_bytes as f32 / (1024.0 * 1024.0 * 1024.0);

        GpuMetrics {
            primary_gpu_name: if primary.name.is_empty() {
                "Graphics Adapter".to_string()
            } else {
                primary.name.clone()
            },
            primary_vendor: primary.vendor.clone(),
            primary_type: primary.gpu_type.clone(),
            primary_vram_used_gb: used_gb,
            primary_vram_total_gb: total_gb.max(0.1),
            primary_vram_usage_percentage: primary.vram_usage_percentage,
            primary_shared_used_gb: shared_used_gb,
            primary_shared_total_gb: shared_total_gb.max(0.1),
            primary_shared_usage_percentage: primary.shared_usage_percentage,
            primary_driver_version: primary.driver_version.clone(),
            primary_gpu_usage_pct: primary.gpu_usage_pct,
            primary_copy_engine_pct: primary.copy_engine_pct,
            primary_video_encode_pct: primary.video_encode_pct,
            primary_video_decode_pct: primary.video_decode_pct,
            primary_gpu_clock_mhz: primary.gpu_clock_mhz,
            primary_pcie_gen: primary.pcie_gen,
            primary_pcie_width: primary.pcie_width,
            primary_temperature_c: primary.temperature_c,
            gpus,
        }
    }
}

pub fn discover_registry_gpus() -> Vec<GpuInfo> {
    let mut results = Vec::new();
    unsafe {
        let class_path =
            w!("SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}");
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
                )
                .is_err()
                {
                    break;
                }
                index += 1;

                let sub_str = String::from_utf16_lossy(&subkey_name[..name_len as usize]);
                if sub_str.starts_with("0") {
                    let subkey_wstr = format!("{}\0", sub_str)
                        .encode_utf16()
                        .collect::<Vec<u16>>();
                    let mut sub_hkey = HKEY::default();
                    if RegOpenKeyExW(
                        hkey,
                        PCWSTR(subkey_wstr.as_ptr()),
                        0,
                        KEY_READ,
                        &mut sub_hkey,
                    )
                    .is_ok()
                    {
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
                        )
                        .is_ok()
                            && desc_len > 0
                        {
                            let u16_slice = std::slice::from_raw_parts(
                                desc_buf.as_ptr() as *const u16,
                                (desc_len as usize / 2).saturating_sub(1),
                            );
                            let driver_desc =
                                String::from_utf16_lossy(u16_slice).trim().to_string();

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
                                )
                                .is_ok()
                                    && prov_len > 0
                                {
                                    let prov_u16 = std::slice::from_raw_parts(
                                        prov_buf.as_ptr() as *const u16,
                                        (prov_len as usize / 2).saturating_sub(1),
                                    );
                                    let prov_str =
                                        String::from_utf16_lossy(prov_u16).to_lowercase();
                                    if prov_str.contains("nvidia") {
                                        vendor = "NVIDIA".to_string();
                                    } else if prov_str.contains("amd")
                                        || prov_str.contains("advanced micro")
                                    {
                                        vendor = "AMD".to_string();
                                    } else if prov_str.contains("intel") {
                                        vendor = "Intel".to_string();
                                    }
                                }

                                // Query DriverVersion
                                let mut ver_buf = [0u8; 256];
                                let mut ver_len = ver_buf.len() as u32;
                                let ver_name = w!("DriverVersion");
                                let mut driver_version = String::new();
                                if RegQueryValueExW(
                                    sub_hkey,
                                    ver_name,
                                    None,
                                    None,
                                    Some(ver_buf.as_mut_ptr()),
                                    Some(&mut ver_len),
                                )
                                .is_ok()
                                    && ver_len > 0
                                {
                                    let ver_u16 = std::slice::from_raw_parts(
                                        ver_buf.as_ptr() as *const u16,
                                        (ver_len as usize / 2).saturating_sub(1),
                                    );
                                    driver_version =
                                        String::from_utf16_lossy(ver_u16).trim().to_string();
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
                                    gpu_type: if is_discrete {
                                        "Discrete GPU".to_string()
                                    } else {
                                        "Integrated GPU".to_string()
                                    },
                                    is_active: false,
                                    driver_version,
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

    fn update(
        &mut self,
        snapshot: &mut super::TelemetrySnapshot,
        _config: &crate::config::AppConfig,
    ) {
        snapshot.gpu = self.collect();
    }
}

fn extract_luid_key(name: &str) -> Option<String> {
    let lower = name.to_lowercase();
    if let Some(pos) = lower.find("luid_0x") {
        let sub = &lower[pos..];
        let parts: Vec<&str> = sub.split('_').collect();
        if parts.len() >= 3 {
            return Some(format!("{}_{}_{}", parts[0], parts[1], parts[2]));
        }
    }
    None
}
