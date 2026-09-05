use std::collections::HashMap;
use sysinfo::Networks;
use windows::core::PCWSTR;
use windows::Win32::NetworkManagement::IpHelper::{
    GetAdaptersAddresses, GAA_FLAG_INCLUDE_GATEWAYS, IP_ADAPTER_ADDRESSES_LH,
};

#[derive(Debug, Clone, Default)]
pub struct NetworkAdapterInfo {
    pub name: String,
    pub display_name: String,
    pub ip: String,
    pub download_bytes_sec: u64,
    pub upload_bytes_sec: u64,
    pub total_received: u64,
    pub total_transmitted: u64,
    pub is_up: bool,

    // Advanced Metrics (gated by config.adv_network)
    pub link_speed_bps: u64,
    pub adapter_type: String, // "Wi-Fi 6" | "Ethernet" | "Virtual"
    pub mac_address: String,
    pub packets_recv_per_sec: u64,
    pub packets_sent_per_sec: u64,
    pub errors_recv: u64,
    pub discards_recv: u64,
    pub signal_strength_pct: Option<u8>,
    pub ssid: Option<String>,
    pub gateway: Option<String>,
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

#[derive(Debug, Clone, Default)]
struct AdapterMeta {
    display_name: String,
    adapter_type: String,
    mac_address: String,
    link_speed_bps: u64,
    gateway: Option<String>,
    is_wifi: bool,
}

pub struct NetworkCollector {
    networks: Networks,
    meta_cache: HashMap<String, AdapterMeta>,
    tick: u64,
}

impl NetworkCollector {
    pub fn new() -> Self {
        let mut networks = Networks::new_with_refreshed_list();
        networks.refresh(true);
        let meta_cache = query_adapters_metadata();
        Self {
            networks,
            meta_cache,
            tick: 0,
        }
    }

    pub fn collect(&mut self) -> NetworkMetrics {
        self.tick += 1;
        if self.tick % 30 == 1 || self.meta_cache.is_empty() {
            self.meta_cache = query_adapters_metadata();
        }

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

            // Look up cached metadata
            let meta = self
                .meta_cache
                .iter()
                .find(|(k, _)| {
                    let k_low = k.to_lowercase();
                    let name_low = interface_name.to_lowercase();
                    name_low.contains(&k_low) || k_low.contains(&name_low)
                })
                .map(|(_, v)| v.clone())
                .unwrap_or_default();

            let display_name = if !meta.display_name.is_empty() {
                meta.display_name
            } else {
                interface_name.clone()
            };

            let adapter_type = if !meta.adapter_type.is_empty() {
                meta.adapter_type
            } else if interface_name.to_lowercase().contains("wi-fi")
                || interface_name.to_lowercase().contains("wireless")
                || interface_name.to_lowercase().contains("wlan")
            {
                "Wi-Fi".to_string()
            } else {
                "Ethernet".to_string()
            };

            let rx_pkts = data.packets_received();
            let tx_pkts = data.packets_transmitted();

            adapters.push(NetworkAdapterInfo {
                name: interface_name.clone(),
                display_name,
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
                link_speed_bps: meta.link_speed_bps,
                adapter_type,
                mac_address: meta.mac_address,
                packets_recv_per_sec: rx_pkts,
                packets_sent_per_sec: tx_pkts,
                errors_recv: data.errors_on_received(),
                discards_recv: 0,
                signal_strength_pct: None,
                ssid: None,
                gateway: meta.gateway,
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

fn query_adapters_metadata() -> HashMap<String, AdapterMeta> {
    let mut map = HashMap::new();

    unsafe {
        let mut buf_len = 16384u32;
        let mut buf = vec![0u8; buf_len as usize];

        let flags = GAA_FLAG_INCLUDE_GATEWAYS;
        let mut ret = GetAdaptersAddresses(
            0, // AF_UNSPEC
            flags,
            None,
            Some(buf.as_mut_ptr() as *mut _),
            &mut buf_len,
        );

        if ret == 111 {
            // ERROR_BUFFER_OVERFLOW
            buf = vec![0u8; buf_len as usize];
            ret = GetAdaptersAddresses(
                0,
                flags,
                None,
                Some(buf.as_mut_ptr() as *mut _),
                &mut buf_len,
            );
        }

        if ret == 0 {
            let mut curr = buf.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
            while !curr.is_null() {
                let item = &*curr;

                let friendly_name = if !item.FriendlyName.is_null() {
                    let mut len = 0;
                    while *item.FriendlyName.0.add(len) != 0 {
                        len += 1;
                    }
                    let slice = std::slice::from_raw_parts(item.FriendlyName.0, len);
                    String::from_utf16_lossy(slice)
                } else {
                    String::new()
                };

                let desc = if !item.Description.is_null() {
                    let mut len = 0;
                    while *item.Description.0.add(len) != 0 {
                        len += 1;
                    }
                    let slice = std::slice::from_raw_parts(item.Description.0, len);
                    String::from_utf16_lossy(slice)
                } else {
                    String::new()
                };

                let mac_address = if item.PhysicalAddressLength == 6 {
                    format!(
                        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                        item.PhysicalAddress[0],
                        item.PhysicalAddress[1],
                        item.PhysicalAddress[2],
                        item.PhysicalAddress[3],
                        item.PhysicalAddress[4],
                        item.PhysicalAddress[5]
                    )
                } else {
                    String::new()
                };

                let is_wifi = item.IfType == 71; // IF_TYPE_IEEE80211
                let adapter_type = match item.IfType {
                    71 => "Wi-Fi".to_string(),
                    6 => "Ethernet".to_string(),
                    24 => "Loopback".to_string(),
                    53 => "Virtual".to_string(),
                    _ => "Ethernet".to_string(),
                };

                let link_speed = item.TransmitLinkSpeed.max(item.ReceiveLinkSpeed);

                let meta = AdapterMeta {
                    display_name: if !desc.is_empty() {
                        desc.clone()
                    } else {
                        friendly_name.clone()
                    },
                    adapter_type,
                    mac_address,
                    link_speed_bps: link_speed,
                    gateway: None,
                    is_wifi,
                };

                if !friendly_name.is_empty() {
                    map.insert(friendly_name.clone(), meta.clone());
                }
                if !desc.is_empty() {
                    map.insert(desc, meta);
                }

                curr = item.Next;
            }
        }
    }

    map
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
