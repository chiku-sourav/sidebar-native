use sysinfo::{CpuRefreshKind, RefreshKind, System};

#[derive(Debug, Clone, Default)]
pub struct CpuMetrics {
    pub brand: String,
    pub global_usage: f32,
    pub core_count: usize,
    pub core_usages: Vec<f32>,
    pub frequency_mhz: u64,
}

pub struct CpuCollector {
    sys: System,
}

impl CpuCollector {
    pub fn new() -> Self {
        let sys = System::new_with_specifics(
            RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
        );
        Self { sys }
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

        CpuMetrics {
            brand,
            global_usage,
            core_count: cpus.len(),
            core_usages,
            frequency_mhz,
        }
    }
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
