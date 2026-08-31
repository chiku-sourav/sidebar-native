use std::collections::HashMap;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};
use crate::config::{AppConfig, ProcessSortBy};

#[derive(Debug, Clone, Default)]
pub struct ProcessInfo {
    pub name: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub formatted_memory: String,
    pub disk_read_bytes_sec: u64,
    pub disk_write_bytes_sec: u64,
    pub disk_total_bytes_sec: u64,
}

// Keep ProcessMemoryInfo alias for backwards compatibility if needed
pub type ProcessMemoryInfo = ProcessInfo;

pub struct ProcessCollector {
    sys: System,
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
        Self { sys }
    }

    pub fn collect_top_processes(&mut self, limit: usize, sort_by: ProcessSortBy) -> Vec<ProcessInfo> {
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory()
                .with_disk_usage(),
        );

        struct AggregatedProcess {
            cpu_usage: f32,
            memory_bytes: u64,
            disk_read: u64,
            disk_write: u64,
        }

        let mut aggregated: HashMap<String, AggregatedProcess> = HashMap::new();

        for (_pid, process) in self.sys.processes() {
            let name = process.name().to_string_lossy();
            let clean_name = name.trim_end_matches(".exe").to_string();
            let memory = process.memory();
            let cpu = process.cpu_usage();
            let disk = process.disk_usage();

            let entry = aggregated.entry(clean_name).or_insert(AggregatedProcess {
                cpu_usage: 0.0,
                memory_bytes: 0,
                disk_read: 0,
                disk_write: 0,
            });

            entry.cpu_usage += cpu;
            entry.memory_bytes += memory;
            entry.disk_read += disk.read_bytes;
            entry.disk_write += disk.written_bytes;
        }

        let mut list: Vec<(String, AggregatedProcess)> = aggregated.into_iter().collect();

        match sort_by {
            ProcessSortBy::Cpu => {
                list.sort_by(|a, b| b.1.cpu_usage.partial_cmp(&a.1.cpu_usage).unwrap_or(std::cmp::Ordering::Equal));
            }
            ProcessSortBy::Memory => {
                list.sort_by(|a, b| b.1.memory_bytes.cmp(&a.1.memory_bytes));
            }
            ProcessSortBy::Disk => {
                list.sort_by(|a, b| (b.1.disk_read + b.1.disk_write).cmp(&(a.1.disk_read + a.1.disk_write)));
            }
        }

        list.into_iter()
            .take(limit)
            .map(|(name, agg)| {
                let formatted = format_bytes(agg.memory_bytes);
                ProcessInfo {
                    name,
                    cpu_usage: agg.cpu_usage,
                    memory_bytes: agg.memory_bytes,
                    formatted_memory: formatted,
                    disk_read_bytes_sec: agg.disk_read,
                    disk_write_bytes_sec: agg.disk_write,
                    disk_total_bytes_sec: agg.disk_read + agg.disk_write,
                }
            })
            .collect()
    }
}

impl super::collector::TelemetryCollector for ProcessCollector {
    fn name(&self) -> &'static str {
        "Processes"
    }

    fn update(&mut self, snapshot: &mut super::TelemetrySnapshot, config: &AppConfig) {
        snapshot.top_processes = self.collect_top_processes(6, config.sort_processes_by);
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes as f64;
    if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.0} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

pub fn format_speed(bytes_sec: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes_sec as f64;
    if b >= GB {
        format!("{:.1} GB/s", b / GB)
    } else if b >= MB {
        format!("{:.1} MB/s", b / MB)
    } else if b >= KB {
        format!("{:.0} KB/s", b / KB)
    } else {
        format!("{} B/s", bytes_sec)
    }
}
