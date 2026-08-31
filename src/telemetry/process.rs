use crate::config::{AppConfig, ProcessSortBy};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};
use windows::core::{GUID, PCWSTR};
use windows::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCP6TABLE_OWNER_PID,
    MIB_TCPROW_OWNER_PID, MIB_TCPTABLE_OWNER_PID, MIB_UDP6ROW_OWNER_PID, MIB_UDP6TABLE_OWNER_PID,
    MIB_UDPROW_OWNER_PID, MIB_UDPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID,
};
use windows::Win32::System::Diagnostics::Etw::{
    CloseTrace, ControlTraceW, EnableTraceEx2, OpenTraceW, ProcessTrace, StartTraceW,
    CONTROLTRACE_HANDLE, EVENT_RECORD, EVENT_TRACE_CONTROL_STOP, EVENT_TRACE_LOGFILEW,
    EVENT_TRACE_PROPERTIES, EVENT_TRACE_REAL_TIME_MODE, PROCESS_TRACE_MODE_EVENT_RECORD,
    PROCESS_TRACE_MODE_REAL_TIME,
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
    pub net_rx_bytes_sec: u64,
    pub net_tx_bytes_sec: u64,
    pub net_total_bytes_sec: u64,
    pub tcp_sockets: usize,
    pub tcp_established: usize,
    pub tcp_listening: usize,
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
    net_rx: u64,
    net_tx: u64,
    tcp_sockets: usize,
    tcp_established: usize,
    tcp_listening: usize,
    udp_sockets: usize,
    active_sockets: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessSocketStats {
    pub tcp_sockets: usize,
    pub tcp_established: usize,
    pub tcp_listening: usize,
    pub udp_sockets: usize,
    pub total_sockets: usize,
}

// Microsoft-Windows-Kernel-Network GUID: {7dd42a49-5329-4832-8dfd-43d979153a88}
const KERNEL_NETWORK_GUID: GUID = GUID::from_u128(0x7dd42a49_5329_4832_8dfd_43d979153a88);

static ETW_PID_STATS: Mutex<Option<HashMap<u32, (u64, u64)>>> = Mutex::new(None);

unsafe extern "system" fn etw_event_callback(record: *mut EVENT_RECORD) {
    if record.is_null() {
        return;
    }
    let rec = &*record;
    let pid = rec.EventHeader.ProcessId;
    if pid == 0 {
        return;
    }

    let event_id = rec.EventHeader.EventDescriptor.Id;
    let len = rec.UserDataLength as u64;

    // 10 = TcpIpSend, 11 = TcpIpRecv, 12 = TcpIpSendIPv6, 13 = TcpIpRecvIPv6, 42 = UdpSend, 43 = UdpRecv
    let (rx, tx) = match event_id {
        10 | 12 | 42 => (0, if len > 0 { len } else { 64 }),
        11 | 13 | 43 => (if len > 0 { len } else { 64 }, 0),
        _ => return,
    };

    if let Ok(mut guard) = ETW_PID_STATS.lock() {
        if let Some(map) = guard.as_mut() {
            let entry = map.entry(pid).or_insert((0, 0));
            entry.0 += rx;
            entry.1 += tx;
        }
    }
}

pub struct EtwNetworkCollector {
    is_active: bool,
    session_handle: CONTROLTRACE_HANDLE,
    session_name: Vec<u16>,
    last_sample_time: Instant,
}

impl EtwNetworkCollector {
    pub fn is_active(&self) -> bool {
        self.is_active
    }
}

impl EtwNetworkCollector {
    pub fn new() -> Self {
        let session_name_str = "SidebarNativeNetSession\0";
        let session_name: Vec<u16> = session_name_str.encode_utf16().collect();
        let buf_len = std::mem::size_of::<EVENT_TRACE_PROPERTIES>() + 512;
        let mut buffer = vec![0u8; buf_len];
        let properties = buffer.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;

        let mut session_handle = CONTROLTRACE_HANDLE::default();
        let mut is_active = false;

        unsafe {
            (*properties).Wnode.BufferSize = buf_len as u32;
            (*properties).Wnode.Flags = 0x00020000; // WNODE_FLAG_TRACED_GUID
            (*properties).LogFileMode = EVENT_TRACE_REAL_TIME_MODE;
            (*properties).LoggerNameOffset = std::mem::size_of::<EVENT_TRACE_PROPERTIES>() as u32;

            // Stop any stale session with this name
            let _ = ControlTraceW(
                CONTROLTRACE_HANDLE::default(),
                PCWSTR::from_raw(session_name.as_ptr()),
                properties,
                EVENT_TRACE_CONTROL_STOP,
            );

            let start_res = StartTraceW(
                &mut session_handle,
                PCWSTR::from_raw(session_name.as_ptr()),
                properties,
            );
            println!("StartTraceW result: {:?}", start_res);

            if start_res.is_ok() {
                let enable_res = EnableTraceEx2(
                    session_handle,
                    &KERNEL_NETWORK_GUID,
                    1, // EVENT_CONTROL_CODE_ENABLE_PROVIDER
                    4, // TRACE_LEVEL_INFORMATION
                    0xFFFFFFFFFFFFFFFF,
                    0,
                    0,
                    None,
                );
                println!("EnableTraceEx2 result: {:?}", enable_res);

                if enable_res.is_ok() {
                    if let Ok(mut guard) = ETW_PID_STATS.lock() {
                        *guard = Some(HashMap::new());
                    }

                    let sess_name_clone = session_name.clone();
                    std::thread::spawn(move || {
                        let mut logfile = EVENT_TRACE_LOGFILEW::default();
                        logfile.LoggerName =
                            windows::core::PWSTR(sess_name_clone.as_ptr() as *mut u16);
                        logfile.Anonymous1.ProcessTraceMode =
                            PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_EVENT_RECORD;
                        logfile.Anonymous2.EventRecordCallback = Some(etw_event_callback);

                        let trace_handle = OpenTraceW(&mut logfile);
                        println!("OpenTraceW result: {:?}", trace_handle);
                        if trace_handle.Value != 0 && trace_handle.Value != !0 {
                            let handles = [trace_handle];
                            let proc_res = ProcessTrace(&handles, None, None);
                            println!("ProcessTrace result: {:?}", proc_res);
                            let _ = CloseTrace(trace_handle);
                        }
                    });

                    is_active = true;
                } else {
                    let _ = ControlTraceW(
                        session_handle,
                        PCWSTR::from_raw(session_name.as_ptr()),
                        properties,
                        EVENT_TRACE_CONTROL_STOP,
                    );
                }
            }
        }

        Self {
            is_active,
            session_handle,
            session_name,
            last_sample_time: Instant::now(),
        }
    }

    pub fn sample_and_drain(&mut self) -> HashMap<u32, (u64, u64)> {
        if !self.is_active {
            return HashMap::new();
        }

        let now = Instant::now();
        let elapsed = now
            .duration_since(self.last_sample_time)
            .as_secs_f64()
            .max(0.1);
        self.last_sample_time = now;

        let raw_deltas = {
            let mut guard = match ETW_PID_STATS.lock() {
                Ok(g) => g,
                Err(_) => return HashMap::new(),
            };
            if let Some(map) = guard.as_mut() {
                std::mem::take(map)
            } else {
                HashMap::new()
            }
        };

        let mut rates = HashMap::with_capacity(raw_deltas.len());
        for (pid, (rx, tx)) in raw_deltas {
            let rx_sec = (rx as f64 / elapsed).round() as u64;
            let tx_sec = (tx as f64 / elapsed).round() as u64;
            rates.insert(pid, (rx_sec, tx_sec));
        }
        rates
    }
}

impl Drop for EtwNetworkCollector {
    fn drop(&mut self) {
        if self.is_active {
            unsafe {
                let buf_len = std::mem::size_of::<EVENT_TRACE_PROPERTIES>() + 512;
                let mut buffer = vec![0u8; buf_len];
                let properties = buffer.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;
                (*properties).Wnode.BufferSize = buf_len as u32;
                (*properties).LoggerNameOffset =
                    std::mem::size_of::<EVENT_TRACE_PROPERTIES>() as u32;
                let _ = ControlTraceW(
                    self.session_handle,
                    PCWSTR::from_raw(self.session_name.as_ptr()),
                    properties,
                    EVENT_TRACE_CONTROL_STOP,
                );
            }
        }
    }
}

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

        let mut aggregated: HashMap<String, AggregatedProcess> = HashMap::new();

        for process in self.sys.processes().values() {
            let name = process.name().to_string_lossy();
            let clean_name = name.trim_end_matches(".exe").to_string();
            let memory = process.memory();
            let cpu = process.cpu_usage();
            let disk = process.disk_usage();
            let pid = process.pid().as_u32();
            let sockets = socket_counts.get(&pid).copied().unwrap_or_default();
            let (rx_sec, tx_sec) = net_rates.get(&pid).copied().unwrap_or((0, 0));

            let entry = aggregated
                .entry(clean_name)
                .or_insert_with(AggregatedProcess::default);

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

fn to_process_info(name: String, agg: AggregatedProcess) -> ProcessInfo {
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

pub fn collect_process_socket_counts() -> HashMap<u32, ProcessSocketStats> {
    let mut map: HashMap<u32, ProcessSocketStats> = HashMap::new();

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
                    let entry = map.entry(row.dwOwningPid).or_default();
                    entry.tcp_sockets += 1;
                    entry.total_sockets += 1;
                    match row.dwState {
                        5 => entry.tcp_established += 1, // MIB_TCP_STATE_ESTAB
                        2 => entry.tcp_listening += 1,   // MIB_TCP_STATE_LISTEN
                        _ => {}
                    }
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
                    let entry = map.entry(row.dwOwningPid).or_default();
                    entry.tcp_sockets += 1;
                    entry.total_sockets += 1;
                    match row.dwState {
                        5 => entry.tcp_established += 1, // MIB_TCP_STATE_ESTAB
                        2 => entry.tcp_listening += 1,   // MIB_TCP_STATE_LISTEN
                        _ => {}
                    }
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
                    let entry = map.entry(row.dwOwningPid).or_default();
                    entry.udp_sockets += 1;
                    entry.total_sockets += 1;
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
                    let entry = map.entry(row.dwOwningPid).or_default();
                    entry.udp_sockets += 1;
                    entry.total_sockets += 1;
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
        snapshot.etw_network_active = self.etw.is_active();

        snapshot.top_processes = match config.sort_processes_by {
            ProcessSortBy::Cpu => snapshot.top_cpu_processes.clone(),
            ProcessSortBy::Memory => snapshot.top_ram_processes.clone(),
            ProcessSortBy::Disk => snapshot.top_disk_processes.clone(),
            ProcessSortBy::Network => snapshot.top_network_processes.clone(),
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
