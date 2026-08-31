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

pub type ProcessMemoryInfo = ProcessInfo;

#[derive(Debug, Clone, Default)]
pub struct TopProcessLists {
    pub cpu: Vec<ProcessInfo>,
    pub ram: Vec<ProcessInfo>,
    pub disk: Vec<ProcessInfo>,
    pub network: Vec<ProcessInfo>,
}

#[derive(Clone, Default)]
pub struct AggregatedProcess {
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub disk_read: u64,
    pub disk_write: u64,
    pub net_rx: u64,
    pub net_tx: u64,
    pub tcp_sockets: usize,
    pub tcp_established: usize,
    pub tcp_listening: usize,
    pub udp_sockets: usize,
    pub active_sockets: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessSocketStats {
    pub tcp_sockets: usize,
    pub tcp_established: usize,
    pub tcp_listening: usize,
    pub udp_sockets: usize,
    pub total_sockets: usize,
}

