use crate::config::{AppConfig, ProcessSortBy};
use std::collections::HashMap;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};
use windows::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCP6TABLE_OWNER_PID,
    MIB_TCPROW_OWNER_PID, MIB_TCPTABLE_OWNER_PID, MIB_UDP6ROW_OWNER_PID,
    MIB_UDP6TABLE_OWNER_PID, MIB_UDPROW_OWNER_PID, MIB_UDPTABLE_OWNER_PID,
    TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID,
};

#[derive(Debug, Clone, Default)]
pub struct ProcessInfo {
    pub name: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub formatted_memory: String,
    pub disk_read_bytes_sec: u64,
    pub disk_write_bytes_sec: u64,
    pub disk_total_bytes_sec: u64,
    pub tcp_sockets: usize,
    pub udp_sockets: usize,
    pub active_sockets: usize,
}

// Keep ProcessMemoryInfo alias for backwards compatibility if needed
pub type ProcessMemoryInfo = ProcessInfo;

#[derive(Debug, Clone, Default)]
pub struct TopProcessLists {
    pub cpu: Vec<ProcessInfo>,
    pub ram: Vec<ProcessInfo>,
    pub disk: Vec<ProcessInfo>,
    pub network: Vec<ProcessInfo>,
}

#[derive(Clone, Default)]
struct AggregatedProcess {
    cpu_usage: f32,
    memory_bytes: u64,
    disk_read: u64,
    disk_write: u64,
    tcp_sockets: usize,
    udp_sockets: usize,
    active_sockets: usize,
}

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

        let mut aggregated: HashMap<String, AggregatedProcess> = HashMap::new();

        for process in self.sys.processes().values() {
            let name = process.name().to_string_lossy();
            let clean_name = name.trim_end_matches(".exe").to_string();
            let memory = process.memory();
            let cpu = process.cpu_usage();
            let disk = process.disk_usage();
            let pid = process.pid().as_u32();
            let (tcp, udp) = socket_counts.get(&pid).copied().unwrap_or((0, 0));

            let entry = aggregated.entry(clean_name).or_insert_with(AggregatedProcess::default);

            entry.cpu_usage += cpu;
            entry.memory_bytes += memory;
            entry.disk_read += disk.read_bytes;
            entry.disk_write += disk.written_bytes;
            entry.tcp_sockets += tcp;
            entry.udp_sockets += udp;
            entry.active_sockets += tcp + udp;
        }

        let base_list: Vec<(String, AggregatedProcess)> = aggregated.into_iter().collect();

        // 1. Top CPU
        let mut cpu_list = base_list.clone();
        cpu_list.sort_by(|a, b| {
            b.1.cpu_usage
                .partial_cmp(&a.1.cpu_usage)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let cpu = cpu_list
            .into_iter()
            .take(limit)
            .map(|(name, agg)| to_process_info(name, agg))
            .collect();

        // 2. Top RAM
        let mut ram_list = base_list.clone();
        ram_list.sort_by_key(|a| std::cmp::Reverse(a.1.memory_bytes));
        let ram = ram_list
            .into_iter()
            .take(limit)
            .map(|(name, agg)| to_process_info(name, agg))
            .collect();

        // 3. Top Disk
        let mut disk_list = base_list.clone();
        disk_list.sort_by(|a, b| {
            (b.1.disk_read + b.1.disk_write).cmp(&(a.1.disk_read + a.1.disk_write))
        });
        let disk = disk_list
            .into_iter()
            .take(limit)
            .map(|(name, agg)| to_process_info(name, agg))
            .collect();

        // 4. Top Network (Rank by active sockets, then I/O activity)
        let mut net_list = base_list;
        net_list.sort_by(|a, b| {
            let a_has_sockets = a.1.active_sockets > 0;
            let b_has_sockets = b.1.active_sockets > 0;
            b_has_sockets
                .cmp(&a_has_sockets)
                .then_with(|| b.1.active_sockets.cmp(&a.1.active_sockets))
                .then_with(|| (b.1.disk_read + b.1.disk_write).cmp(&(a.1.disk_read + a.1.disk_write)))
                .then_with(|| {
                    b.1.cpu_usage
                        .partial_cmp(&a.1.cpu_usage)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        let network = net_list
            .into_iter()
            .take(limit)
            .map(|(name, agg)| to_process_info(name, agg))
            .collect();

        TopProcessLists {
            cpu,
            ram,
            disk,
            network,
        }
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
        }
    }
}

fn to_process_info(name: String, agg: AggregatedProcess) -> ProcessInfo {
    let formatted = format_bytes(agg.memory_bytes);
    ProcessInfo {
        name,
        cpu_usage: agg.cpu_usage,
        memory_bytes: agg.memory_bytes,
        formatted_memory: formatted,
        disk_read_bytes_sec: agg.disk_read,
        disk_write_bytes_sec: agg.disk_write,
        disk_total_bytes_sec: agg.disk_read + agg.disk_write,
        tcp_sockets: agg.tcp_sockets,
        udp_sockets: agg.udp_sockets,
        active_sockets: agg.active_sockets,
    }
}

pub fn collect_process_socket_counts() -> HashMap<u32, (usize, usize)> {
    let mut map: HashMap<u32, (usize, usize)> = HashMap::new();

    unsafe {
        // 1. IPv4 TCP Table (AF_INET = 2)
        let mut size = 0u32;
        let _ = GetExtendedTcpTable(None, &mut size, false, 2, TCP_TABLE_OWNER_PID_ALL, 0);
        if size > 0 {
            let mut buf = vec![0u8; size as usize];
            if GetExtendedTcpTable(
                Some(buf.as_mut_ptr() as *mut _),
                &mut size,
                false,
                2,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            ) == 0
            {
                let table = &*(buf.as_ptr() as *const MIB_TCPTABLE_OWNER_PID);
                let count = table.dwNumEntries as usize;
                let rows = std::slice::from_raw_parts(table.table.as_ptr(), count);
                for row in rows {
                    let entry = map.entry(row.dwOwningPid).or_insert((0, 0));
                    entry.0 += 1;
                }
            }
        }

        // 2. IPv6 TCP Table (AF_INET6 = 23)
        let mut size6 = 0u32;
        let _ = GetExtendedTcpTable(None, &mut size6, false, 23, TCP_TABLE_OWNER_PID_ALL, 0);
        if size6 > 0 {
            let mut buf = vec![0u8; size6 as usize];
            if GetExtendedTcpTable(
                Some(buf.as_mut_ptr() as *mut _),
                &mut size6,
                false,
                23,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            ) == 0
            {
                let table = &*(buf.as_ptr() as *const MIB_TCP6TABLE_OWNER_PID);
                let count = table.dwNumEntries as usize;
                let rows = std::slice::from_raw_parts(table.table.as_ptr(), count);
                for row in rows {
                    let entry = map.entry(row.dwOwningPid).or_insert((0, 0));
                    entry.0 += 1;
                }
            }
        }

        // 3. IPv4 UDP Table (AF_INET = 2)
        let mut udp_size = 0u32;
        let _ = GetExtendedUdpTable(None, &mut udp_size, false, 2, UDP_TABLE_OWNER_PID, 0);
        if udp_size > 0 {
            let mut buf = vec![0u8; udp_size as usize];
            if GetExtendedUdpTable(
                Some(buf.as_mut_ptr() as *mut _),
                &mut udp_size,
                false,
                2,
                UDP_TABLE_OWNER_PID,
                0,
            ) == 0
            {
                let table = &*(buf.as_ptr() as *const MIB_UDPTABLE_OWNER_PID);
                let count = table.dwNumEntries as usize;
                let rows = std::slice::from_raw_parts(table.table.as_ptr(), count);
                for row in rows {
                    let entry = map.entry(row.dwOwningPid).or_insert((0, 0));
                    entry.1 += 1;
                }
            }
        }

        // 4. IPv6 UDP Table (AF_INET6 = 23)
        let mut udp6_size = 0u32;
        let _ = GetExtendedUdpTable(None, &mut udp6_size, false, 23, UDP_TABLE_OWNER_PID, 0);
        if udp6_size > 0 {
            let mut buf = vec![0u8; udp6_size as usize];
            if GetExtendedUdpTable(
                Some(buf.as_mut_ptr() as *mut _),
                &mut udp6_size,
                false,
                23,
                UDP_TABLE_OWNER_PID,
                0,
            ) == 0
            {
                let table = &*(buf.as_ptr() as *const MIB_UDP6TABLE_OWNER_PID);
                let count = table.dwNumEntries as usize;
                let rows = std::slice::from_raw_parts(table.table.as_ptr(), count);
                for row in rows {
                    let entry = map.entry(row.dwOwningPid).or_insert((0, 0));
                    entry.1 += 1;
                }
            }
        }
    }

    map
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

        snapshot.top_processes = match config.sort_processes_by {
            ProcessSortBy::Cpu => snapshot.top_cpu_processes.clone(),
            ProcessSortBy::Memory => snapshot.top_ram_processes.clone(),
            ProcessSortBy::Disk => snapshot.top_disk_processes.clone(),
        };
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

