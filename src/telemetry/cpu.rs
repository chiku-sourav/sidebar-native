use sysinfo::{CpuRefreshKind, RefreshKind, System};
use windows::core::w;
use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ,
    REG_VALUE_TYPE,
};
use windows::Win32::System::SystemInformation::{
    GetLogicalProcessorInformationEx, RelationProcessorPackage,
    SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
};

use super::pdh::{PdhHCounter, PdhHelper};

#[derive(Debug, Clone, Default)]
pub struct CpuMetrics {
    pub brand: String,
    pub global_usage: f32,
    pub core_count: usize,
    pub core_usages: Vec<f32>,
    pub frequency_mhz: u64,

    // Advanced Metrics (gated by config.adv_cpu)
    pub physical_core_count: usize,
    pub socket_count: usize,
    pub base_clock_mhz: u64,
    pub boost_clock_mhz: u64,
    pub user_pct: f32,
    pub privileged_pct: f32,
    pub context_switches_per_sec: u64,
    pub interrupts_per_sec: u64,
}

pub struct CpuCollector {
    sys: System,
    physical_core_count: usize,
    socket_count: usize,
    base_clock_mhz: u64,
    pdh: Option<PdhHelper>,
    h_user_pct: PdhHCounter,
    h_priv_pct: PdhHCounter,
    h_ctx_sw: PdhHCounter,
    h_irq: PdhHCounter,
    h_freq: PdhHCounter,
}

impl CpuCollector {
    pub fn new() -> Self {
        let sys = System::new_with_specifics(
            RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
        );

        let physical_core_count = sys
            .physical_core_count()
            .unwrap_or_else(|| sys.cpus().len());
        let socket_count = count_processor_sockets();
        let base_clock_mhz = read_base_clock_registry();

        let mut pdh = PdhHelper::new();
        let mut h_user_pct = 0;
        let mut h_priv_pct = 0;
        let mut h_ctx_sw = 0;
        let mut h_irq = 0;
        let mut h_freq = 0;

        if let Some(p) = pdh.as_mut() {
            h_user_pct = p.add_counter("\\Processor(_Total)\\% User Time");
            h_priv_pct = p.add_counter("\\Processor(_Total)\\% Privileged Time");
            h_ctx_sw = p.add_counter("\\System\\Context Switches/sec");
            h_irq = p.add_counter("\\Processor(_Total)\\Interrupts/sec");
            h_freq = p.add_counter("\\Processor Information(_Total)\\Processor Frequency");
        }

        Self {
            sys,
            physical_core_count,
            socket_count,
            base_clock_mhz,
            pdh,
            h_user_pct,
            h_priv_pct,
            h_ctx_sw,
            h_irq,
            h_freq,
        }
    }

    pub fn collect(&mut self) -> CpuMetrics {
        self.sys.refresh_cpu_usage();

        let cpus = self.sys.cpus();
        let global_usage = self.sys.global_cpu_usage();
        let brand = if let Some(first) = cpus.first() {
            first.brand().trim().to_string()
        } else {
            "Processor".to_string()
        };

        let frequency_mhz = cpus.first().map(|c| c.frequency()).unwrap_or(0);
        let core_usages: Vec<f32> = cpus.iter().map(|c| c.cpu_usage()).collect();

        let mut user_pct = 0.0f32;
        let mut privileged_pct = 0.0f32;
        let mut context_switches_per_sec = 0u64;
        let mut interrupts_per_sec = 0u64;
        let mut boost_clock_mhz = frequency_mhz;

        if let Some(p) = self.pdh.as_mut() {
            if p.collect() {
                user_pct = p.read_f32(self.h_user_pct).clamp(0.0, 100.0);
                privileged_pct = p.read_f32(self.h_priv_pct).clamp(0.0, 100.0);
                context_switches_per_sec = p.read_u64(self.h_ctx_sw);
                interrupts_per_sec = p.read_u64(self.h_irq);
                let pdh_freq = p.read_u64(self.h_freq);
                if pdh_freq > 0 {
                    boost_clock_mhz = pdh_freq;
                }
            }
        }

        if boost_clock_mhz == 0 {
            boost_clock_mhz = self.base_clock_mhz.max(frequency_mhz);
        }

        CpuMetrics {
            brand,
            global_usage,
            core_count: cpus.len(),
            core_usages,
            frequency_mhz,
            physical_core_count: self.physical_core_count,
            socket_count: self.socket_count,
            base_clock_mhz: self.base_clock_mhz,
            boost_clock_mhz,
            user_pct,
            privileged_pct,
            context_switches_per_sec,
            interrupts_per_sec,
        }
    }
}

fn read_base_clock_registry() -> u64 {
    unsafe {
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            w!("HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0"),
            0,
            KEY_READ,
            &mut hkey,
        )
        .is_ok()
        {
            let mut mhz = 0u32;
            let mut len = std::mem::size_of::<u32>() as u32;
            let mut val_type = REG_VALUE_TYPE::default();
            let res = RegQueryValueExW(
                hkey,
                w!("~MHz"),
                None,
                Some(&mut val_type),
                Some(&mut mhz as *mut _ as *mut u8),
                Some(&mut len),
            );
            let _ = RegCloseKey(hkey);
            if res.is_ok() && mhz > 0 {
                return mhz as u64;
            }
        }
    }
    0
}

fn count_processor_sockets() -> usize {
    unsafe {
        let mut returned_len = 0u32;
        let _ = GetLogicalProcessorInformationEx(RelationProcessorPackage, None, &mut returned_len);
        if returned_len > 0 {
            let mut buffer = vec![0u8; returned_len as usize];
            if GetLogicalProcessorInformationEx(
                RelationProcessorPackage,
                Some(buffer.as_mut_ptr() as *mut SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX),
                &mut returned_len,
            )
            .is_ok()
            {
                let mut count = 0;
                let mut offset = 0;
                while offset < returned_len as usize {
                    let ptr = buffer.as_ptr().add(offset)
                        as *const SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX;
                    let size = (*ptr).Size as usize;
                    if size == 0 {
                        break;
                    }
                    count += 1;
                    offset += size;
                }
                if count > 0 {
                    return count;
                }
            }
        }
    }
    1
}

impl super::collector::TelemetryCollector for CpuCollector {
    fn name(&self) -> &'static str {
        "CPU"
    }

    fn update(
        &mut self,
        snapshot: &mut super::TelemetrySnapshot,
        _config: &crate::config::AppConfig,
    ) {
        snapshot.cpu = self.collect();
    }
}
