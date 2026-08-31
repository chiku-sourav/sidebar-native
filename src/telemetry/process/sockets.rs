use std::collections::HashMap;
use windows::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6TABLE_OWNER_PID, MIB_TCPTABLE_OWNER_PID,
    MIB_UDP6TABLE_OWNER_PID, MIB_UDPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID,
};

use super::types::ProcessSocketStats;

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
