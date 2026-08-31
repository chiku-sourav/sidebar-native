pub mod aggregator;
pub mod etw;
pub mod sockets;
pub mod types;

pub use aggregator::{aggregate_and_rank_processes, to_process_info};
pub use etw::EtwNetworkCollector;
pub use sockets::collect_process_socket_counts;
pub use types::{
    AggregatedProcess, ProcessInfo, ProcessMemoryInfo, ProcessSocketStats, TopProcessLists,
};

pub use crate::telemetry::formatters::{format_bytes, format_speed};

use crate::config::{AppConfig, ProcessSortBy};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

pub struct ProcessCollector {
    sys: System,
    etw: EtwNetworkCollector,
}

impl ProcessCollector {
    pub fn new() -> Self {
        let sys = System::new_with_specifics(
            RefreshKind::nothing().with_processes(
                ProcessRefreshKind::nothing()
                    .with_cpu()
                    .with_memory()
                    .with_disk_usage(),
            ),
        );
        let etw = EtwNetworkCollector::new();
        Self { sys, etw }
    }

    pub fn collect_all_top_processes(&mut self, limit: usize) -> TopProcessLists {
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory()
                .with_disk_usage(),
        );

        let socket_counts = collect_process_socket_counts();
        let net_rates = self.etw.sample_and_drain();

        aggregate_and_rank_processes(&self.sys, &socket_counts, &net_rates, limit)
    }

    pub fn collect_top_processes(
        &mut self,
        limit: usize,
        sort_by: ProcessSortBy,
    ) -> Vec<ProcessInfo> {
        let lists = self.collect_all_top_processes(limit);
        match sort_by {
            ProcessSortBy::Cpu => lists.cpu,
            ProcessSortBy::Memory => lists.ram,
            ProcessSortBy::Disk => lists.disk,
            ProcessSortBy::Network => lists.network,
        }
    }
}

impl super::collector::TelemetryCollector for ProcessCollector {
    fn name(&self) -> &'static str {
        "Processes"
    }

    fn update(&mut self, snapshot: &mut super::TelemetrySnapshot, config: &AppConfig) {
        let limit = config.process_limit_per_category.max(1);
        let lists = self.collect_all_top_processes(limit);
        snapshot.top_cpu_processes = lists.cpu;
        snapshot.top_ram_processes = lists.ram;
        snapshot.top_disk_processes = lists.disk;
        snapshot.top_network_processes = lists.network;
        snapshot.etw_network_active = self.etw.is_active();

        snapshot.top_processes = match config.sort_processes_by {
            ProcessSortBy::Cpu => snapshot.top_cpu_processes.clone(),
            ProcessSortBy::Memory => snapshot.top_ram_processes.clone(),
            ProcessSortBy::Disk => snapshot.top_disk_processes.clone(),
            ProcessSortBy::Network => snapshot.top_network_processes.clone(),
        };
    }
}
