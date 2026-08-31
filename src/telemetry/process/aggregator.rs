use std::collections::HashMap;
use sysinfo::System;

use super::types::{AggregatedProcess, ProcessInfo, ProcessSocketStats, TopProcessLists};
use crate::telemetry::formatters::format_bytes;

pub fn aggregate_and_rank_processes(
    sys: &System,
    socket_counts: &HashMap<u32, ProcessSocketStats>,
    net_rates: &HashMap<u32, (u64, u64)>,
    limit: usize,
) -> TopProcessLists {
    let mut aggregated: HashMap<String, AggregatedProcess> = HashMap::new();

    for process in sys.processes().values() {
        let name = process.name().to_string_lossy();
        let clean_name = name.trim_end_matches(".exe").to_string();
        let memory = process.memory();
        let cpu = process.cpu_usage();
        let disk = process.disk_usage();
        let pid = process.pid().as_u32();
        let sockets = socket_counts.get(&pid).copied().unwrap_or_default();
        let (rx_sec, tx_sec) = net_rates.get(&pid).copied().unwrap_or((0, 0));

        let entry = aggregated.entry(clean_name).or_default();

        entry.cpu_usage += cpu;
        entry.memory_bytes += memory;
        entry.disk_read += disk.read_bytes;
        entry.disk_write += disk.written_bytes;
        entry.net_rx += rx_sec;
        entry.net_tx += tx_sec;
        entry.tcp_sockets += sockets.tcp_sockets;
        entry.tcp_established += sockets.tcp_established;
        entry.tcp_listening += sockets.tcp_listening;
        entry.udp_sockets += sockets.udp_sockets;
        entry.active_sockets += sockets.total_sockets;
    }

    let num_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .max(1) as f32;

    for entry in aggregated.values_mut() {
        entry.cpu_usage = (entry.cpu_usage / num_cores).min(100.0);
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

    // 4. Top Network (Rank by real-time network throughput, then established sockets, then active sockets, then I/O activity)
    let mut net_list = base_list;
    net_list.sort_by(|a, b| {
        let a_net = a.1.net_rx + a.1.net_tx;
        let b_net = b.1.net_rx + b.1.net_tx;
        if a_net > 0 || b_net > 0 {
            return b_net.cmp(&a_net);
        }

        if a.1.tcp_established != b.1.tcp_established {
            return b.1.tcp_established.cmp(&a.1.tcp_established);
        }

        if a.1.active_sockets != b.1.active_sockets {
            return b.1.active_sockets.cmp(&a.1.active_sockets);
        }

        let a_io = a.1.disk_read + a.1.disk_write;
        let b_io = b.1.disk_read + b.1.disk_write;
        if a_io != b_io {
            return b_io.cmp(&a_io);
        }

        b.1.cpu_usage
            .partial_cmp(&a.1.cpu_usage)
            .unwrap_or(std::cmp::Ordering::Equal)
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

pub fn to_process_info(name: String, agg: AggregatedProcess) -> ProcessInfo {
    let formatted = format_bytes(agg.memory_bytes);
    let net_total = agg.net_rx + agg.net_tx;
    ProcessInfo {
        name,
        cpu_usage: agg.cpu_usage,
        memory_bytes: agg.memory_bytes,
        formatted_memory: formatted,
        disk_read_bytes_sec: agg.disk_read,
        disk_write_bytes_sec: agg.disk_write,
        disk_total_bytes_sec: agg.disk_read + agg.disk_write,
        net_rx_bytes_sec: agg.net_rx,
        net_tx_bytes_sec: agg.net_tx,
        net_total_bytes_sec: net_total,
        tcp_sockets: agg.tcp_sockets,
        tcp_established: agg.tcp_established,
        tcp_listening: agg.tcp_listening,
        udp_sockets: agg.udp_sockets,
        active_sockets: agg.active_sockets,
    }
}

