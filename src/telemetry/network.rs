use sysinfo::Networks;

#[derive(Debug, Clone, Default)]
pub struct NetworkAdapterInfo {
    pub name: String,
    pub ip: String,
    pub download_bytes_sec: u64,
    pub upload_bytes_sec: u64,
    pub total_received: u64,
    pub total_transmitted: u64,
    pub is_up: bool,
}

#[derive(Debug, Clone, Default)]
pub struct NetworkMetrics {
    pub download_bytes_sec: u64,
    pub upload_bytes_sec: u64,
    pub primary_interface: String,
    pub local_ip: String,
    pub total_received: u64,
    pub total_transmitted: u64,
    pub adapters: Vec<NetworkAdapterInfo>,
}

pub struct NetworkCollector {
    networks: Networks,
}

impl NetworkCollector {
    pub fn new() -> Self {
        let mut networks = Networks::new_with_refreshed_list();
        networks.refresh(true);
        Self { networks }
    }

    pub fn collect(&mut self) -> NetworkMetrics {
        self.networks.refresh(true);

        let mut total_rx_sec: u64 = 0;
        let mut total_tx_sec: u64 = 0;
        let mut total_rx: u64 = 0;
        let mut total_tx: u64 = 0;
        let mut primary_interface = "Network".to_string();
        let mut primary_ip = String::new();
        let mut max_traffic: u64 = 0;
        let mut adapters = Vec::new();

        for (interface_name, data) in &self.networks {
            let rx = data.received();
            let tx = data.transmitted();
            let traffic = rx + tx;

            total_rx_sec += rx;
            total_tx_sec += tx;
            total_rx += data.total_received();
            total_tx += data.total_transmitted();

            let mut adapter_ip = String::new();
            for ip_net in data.ip_networks() {
                if let std::net::IpAddr::V4(v4) = ip_net.addr {
                    if !v4.is_loopback() {
                        adapter_ip = v4.to_string();
                        if primary_ip.is_empty() || traffic > 0 {
                            primary_ip = v4.to_string();
                        }
                    }
                }
            }

            if traffic > max_traffic {
                max_traffic = traffic;
                primary_interface = interface_name.clone();
            }

            let is_up = !adapter_ip.is_empty() || traffic > 0 || data.total_received() > 0;

            adapters.push(NetworkAdapterInfo {
                name: interface_name.clone(),
                ip: if adapter_ip.is_empty() {
                    "Disconnected".to_string()
                } else {
                    adapter_ip
                },
                download_bytes_sec: rx,
                upload_bytes_sec: tx,
                total_received: data.total_received(),
                total_transmitted: data.total_transmitted(),
                is_up,
            });
        }

        if primary_ip.is_empty() {
            primary_ip = "127.0.0.1".to_string();
        }

        // Sort adapters so active with traffic are first
        adapters.sort_by(|a, b| {
            let traffic_b = b.download_bytes_sec + b.upload_bytes_sec;
            let traffic_a = a.download_bytes_sec + a.upload_bytes_sec;
            traffic_b
                .cmp(&traffic_a)
                .then_with(|| b.is_up.cmp(&a.is_up))
        });

        NetworkMetrics {
            download_bytes_sec: total_rx_sec,
            upload_bytes_sec: total_tx_sec,
            primary_interface,
            local_ip: primary_ip,
            total_received: total_rx,
            total_transmitted: total_tx,
            adapters,
        }
    }
}

impl super::collector::TelemetryCollector for NetworkCollector {
    fn name(&self) -> &'static str {
        "Network"
    }

    fn update(
        &mut self,
        snapshot: &mut super::TelemetrySnapshot,
        _config: &crate::config::AppConfig,
    ) {
        snapshot.network = self.collect();
    }
}
