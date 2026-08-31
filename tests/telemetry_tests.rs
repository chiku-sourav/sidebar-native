use sidebar_native::config::{
    AppConfig, AppTheme, BackdropEffect, FontSize, ProcessSortBy, TemperatureUnit,
    WindowWidthPreset,
};
use sidebar_native::telemetry::process::EtwNetworkCollector;

use sidebar_native::telemetry::network::NetworkAdapterInfo;
use sidebar_native::telemetry::process::{format_bytes, format_speed, ProcessInfo};
use sidebar_native::telemetry::storage::DriveInfo;

#[test]
fn test_config_defaults_and_presets() {
    let cfg = AppConfig::default();
    assert_eq!(cfg.width_preset, WindowWidthPreset::Wide);
    assert_eq!(cfg.sidebar_width, 480);
    assert_eq!(cfg.sort_processes_by, ProcessSortBy::Cpu);
    assert_eq!(cfg.temperature_unit, TemperatureUnit::Celsius);
    assert_eq!(cfg.font_size, FontSize::Large);
    assert_eq!(cfg.theme, AppTheme::Auto);
    assert_eq!(cfg.backdrop, BackdropEffect::Mica);

    assert_eq!(WindowWidthPreset::Compact.base_width(), 350);
    assert_eq!(WindowWidthPreset::Standard.base_width(), 410);
    assert_eq!(WindowWidthPreset::Wide.base_width(), 490);
    assert_eq!(WindowWidthPreset::UltraWide.base_width(), 580);
}

#[test]
fn test_config_json_serialization_roundtrip() {
    let cfg = AppConfig {
        sidebar_width: 520,
        width_preset: WindowWidthPreset::UltraWide,
        sort_processes_by: ProcessSortBy::Disk,
        show_top_cpu: true,
        show_top_ram: true,
        show_top_disk: false,
        show_top_network: true,
        process_limit_per_category: 6,
        ..Default::default()
    };

    let json_str = serde_json::to_string_pretty(&cfg).expect("Serialization failed");
    assert!(json_str.contains("UltraWide"));
    assert!(json_str.contains("Disk"));
    assert!(json_str.contains("show_top_network"));

    let deserialized: AppConfig = serde_json::from_str(&json_str).expect("Deserialization failed");
    assert_eq!(deserialized.sidebar_width, 520);
    assert_eq!(deserialized.width_preset, WindowWidthPreset::UltraWide);
    assert_eq!(deserialized.sort_processes_by, ProcessSortBy::Disk);
    assert!(deserialized.show_top_cpu);
    assert!(deserialized.show_top_ram);
    assert!(!deserialized.show_top_disk);
    assert!(deserialized.show_top_network);
    assert_eq!(deserialized.process_limit_per_category, 6);
}

#[test]
fn test_font_scale_factors() {
    assert!((FontSize::Small.scale() - 1.00).abs() < 0.001);
    assert!((FontSize::Medium.scale() - 1.22).abs() < 0.001);
    assert!((FontSize::Large.scale() - 1.45).abs() < 0.001);
    assert!((FontSize::ExtraLarge.scale() - 1.70).abs() < 0.001);
    assert!((FontSize::Huge.scale() - 2.00).abs() < 0.001);
}

#[test]
fn test_format_bytes_and_speeds() {
    assert_eq!(format_bytes(512), "512 B");
    assert_eq!(format_bytes(1024 * 512), "512 KB");
    assert_eq!(format_bytes(1024 * 1024 * 250), "250 MB");
    assert_eq!(format_bytes(1024 * 1024 * 1024 * 4), "4.0 GB");

    assert_eq!(format_speed(400), "400 B/s");
    assert_eq!(format_speed(1024 * 350), "350 KB/s");
    assert_eq!(format_speed(1024 * 1024 * 24), "24.0 MB/s");
    assert_eq!(format_speed(1024 * 1024 * 1024 * 2), "2.0 GB/s");
}

#[test]
fn test_process_sorting_logic() {
    let mut procs = [
        ProcessInfo {
            name: "chrome".to_string(),
            cpu_usage: 12.5,
            memory_bytes: 1024 * 1024 * 800, // 800 MB
            formatted_memory: "800 MB".to_string(),
            disk_read_bytes_sec: 1024 * 100,
            disk_write_bytes_sec: 1024 * 50,
            disk_total_bytes_sec: 1024 * 150,
            tcp_sockets: 24,
            udp_sockets: 4,
            active_sockets: 28,
            ..Default::default()
        },
        ProcessInfo {
            name: "code".to_string(),
            cpu_usage: 28.0,
            memory_bytes: 1024 * 1024 * 400, // 400 MB
            formatted_memory: "400 MB".to_string(),
            disk_read_bytes_sec: 1024 * 10,
            disk_write_bytes_sec: 1024 * 10,
            disk_total_bytes_sec: 1024 * 20,
            tcp_sockets: 2,
            udp_sockets: 0,
            active_sockets: 2,
            ..Default::default()
        },
        ProcessInfo {
            name: "rustc".to_string(),
            cpu_usage: 4.2,
            memory_bytes: 1024 * 1024 * 1200, // 1200 MB
            formatted_memory: "1.2 GB".to_string(),
            disk_read_bytes_sec: 1024 * 1024 * 5,
            disk_write_bytes_sec: 1024 * 1024 * 10,
            disk_total_bytes_sec: 1024 * 1024 * 15,
            tcp_sockets: 0,
            udp_sockets: 0,
            active_sockets: 0,
            ..Default::default()
        },
    ];

    // 1. Sort by CPU
    procs.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap());
    assert_eq!(procs[0].name, "code"); // 28.0%

    // 2. Sort by Memory
    procs.sort_by_key(|a| std::cmp::Reverse(a.memory_bytes));
    assert_eq!(procs[0].name, "rustc"); // 1200 MB

    // 3. Sort by Disk I/O
    procs.sort_by_key(|a| std::cmp::Reverse(a.disk_total_bytes_sec));
    assert_eq!(procs[0].name, "rustc"); // 15 MB/s

    // 4. Sort by Network Sockets & Activity
    procs.sort_by(|a, b| {
        let a_has = a.active_sockets > 0;
        let b_has = b.active_sockets > 0;
        b_has
            .cmp(&a_has)
            .then_with(|| b.active_sockets.cmp(&a.active_sockets))
            .then_with(|| b.disk_total_bytes_sec.cmp(&a.disk_total_bytes_sec))
    });
    assert_eq!(procs[0].name, "chrome"); // 28 sockets
    assert_eq!(procs[1].name, "code"); // 2 sockets
    assert_eq!(procs[2].name, "rustc"); // 0 sockets
}

#[test]
fn test_telemetry_snapshot_separate_process_lists() {
    use sidebar_native::telemetry::TelemetrySnapshot;

    let mut snapshot = TelemetrySnapshot::default();
    snapshot.top_cpu_processes.push(ProcessInfo {
        name: "ffmpeg".to_string(),
        cpu_usage: 85.0,
        memory_bytes: 500_000_000,
        formatted_memory: "500 MB".to_string(),
        disk_read_bytes_sec: 10_000_000,
        disk_write_bytes_sec: 15_000_000,
        disk_total_bytes_sec: 25_000_000,
        tcp_sockets: 0,
        udp_sockets: 0,
        active_sockets: 0,
        ..Default::default()
    });
    snapshot.top_ram_processes.push(ProcessInfo {
        name: "vmsrv".to_string(),
        cpu_usage: 5.0,
        memory_bytes: 4_000_000_000,
        formatted_memory: "4.0 GB".to_string(),
        disk_read_bytes_sec: 1_000,
        disk_write_bytes_sec: 1_000,
        disk_total_bytes_sec: 2_000,
        tcp_sockets: 2,
        udp_sockets: 1,
        active_sockets: 3,
        ..Default::default()
    });
    snapshot.top_disk_processes.push(ProcessInfo {
        name: "robocopy".to_string(),
        cpu_usage: 12.0,
        memory_bytes: 80_000_000,
        formatted_memory: "80 MB".to_string(),
        disk_read_bytes_sec: 100_000_000,
        disk_write_bytes_sec: 100_000_000,
        disk_total_bytes_sec: 200_000_000,
        tcp_sockets: 0,
        udp_sockets: 0,
        active_sockets: 0,
        ..Default::default()
    });
    snapshot.top_network_processes.push(ProcessInfo {
        name: "discord".to_string(),
        cpu_usage: 2.0,
        memory_bytes: 250_000_000,
        formatted_memory: "250 MB".to_string(),
        disk_read_bytes_sec: 50_000,
        disk_write_bytes_sec: 20_000,
        disk_total_bytes_sec: 70_000,
        tcp_sockets: 18,
        udp_sockets: 6,
        active_sockets: 24,
        ..Default::default()
    });

    assert_eq!(snapshot.top_cpu_processes[0].name, "ffmpeg");
    assert_eq!(snapshot.top_ram_processes[0].name, "vmsrv");
    assert_eq!(snapshot.top_disk_processes[0].name, "robocopy");
    assert_eq!(snapshot.top_network_processes[0].name, "discord");
    assert_eq!(snapshot.top_network_processes[0].active_sockets, 24);
}

#[test]
fn test_drive_media_types_and_wsl_detection() {
    let nvme_drive = DriveInfo {
        letter: "C:".to_string(),
        label: "C: Volume".to_string(),
        drive_type: "NVMe SSD".to_string(),
        model_name: "TS1TMTE400S".to_string(),
        read_bytes_sec: 1024 * 450,
        write_bytes_sec: 1024 * 120,
        total_bytes: 1024 * 1024 * 1024 * 1000,
        free_bytes: 1024 * 1024 * 1024 * 100,
        used_bytes: 1024 * 1024 * 1024 * 900,
        usage_percentage: 90.0,
        is_linux_or_raw: false,
    };

    let linux_sata_drive = DriveInfo {
        letter: "Disk 1".to_string(),
        label: "Linux / Ext4 (Disk 1)".to_string(),
        drive_type: "SATA SSD (Linux / Ext4)".to_string(),
        model_name: "WDC WDS120G2G0A-00JH30".to_string(),
        read_bytes_sec: 1024 * 20,
        write_bytes_sec: 1024 * 5,
        total_bytes: 120_000_000_000,
        free_bytes: 60_000_000_000,
        used_bytes: 60_000_000_000,
        usage_percentage: 50.0,
        is_linux_or_raw: true,
    };

    let wsl_drive = DriveInfo {
        letter: "WSL: Ubuntu".to_string(),
        label: "WSL2 Linux (Ubuntu)".to_string(),
        drive_type: "WSL2 Linux (ext4)".to_string(),
        model_name: "WSL2 Ubuntu Virtual Drive (ext4)".to_string(),
        read_bytes_sec: 1024 * 100,
        write_bytes_sec: 1024 * 30,
        total_bytes: 250_000_000_000,
        free_bytes: 180_000_000_000,
        used_bytes: 70_000_000_000,
        usage_percentage: 28.0,
        is_linux_or_raw: true,
    };

    assert_eq!(nvme_drive.drive_type, "NVMe SSD");
    assert!(linux_sata_drive.is_linux_or_raw);
    assert_eq!(linux_sata_drive.model_name, "WDC WDS120G2G0A-00JH30");
    assert!(wsl_drive.drive_type.contains("ext4"));
}

#[test]
fn test_network_adapters_traffic_ranking() {
    let mut adapters = [
        NetworkAdapterInfo {
            name: "Bluetooth Device".to_string(),
            ip: "Disconnected".to_string(),
            download_bytes_sec: 0,
            upload_bytes_sec: 0,
            total_received: 0,
            total_transmitted: 0,
            is_up: false,
        },
        NetworkAdapterInfo {
            name: "Wi-Fi".to_string(),
            ip: "192.168.29.230".to_string(),
            download_bytes_sec: 1024 * 1024 * 3, // 3 MB/s
            upload_bytes_sec: 1024 * 512,        // 512 KB/s
            total_received: 1024 * 1024 * 1024 * 10,
            total_transmitted: 1024 * 1024 * 1024 * 2,
            is_up: true,
        },
        NetworkAdapterInfo {
            name: "vEthernet (WSL)".to_string(),
            ip: "172.24.48.1".to_string(),
            download_bytes_sec: 1024 * 100,
            upload_bytes_sec: 1024 * 20,
            total_received: 1024 * 1024 * 50,
            total_transmitted: 1024 * 1024 * 10,
            is_up: true,
        },
    ];

    adapters.sort_by(|a, b| {
        let traffic_b = b.download_bytes_sec + b.upload_bytes_sec;
        let traffic_a = a.download_bytes_sec + a.upload_bytes_sec;
        traffic_b
            .cmp(&traffic_a)
            .then_with(|| b.is_up.cmp(&a.is_up))
    });

    assert_eq!(adapters[0].name, "Wi-Fi");
    assert_eq!(adapters[1].name, "vEthernet (WSL)");
    assert_eq!(adapters[2].name, "Bluetooth Device");
}

#[test]
fn test_battery_metrics_and_health_calculations() {
    use sidebar_native::telemetry::power::{BatteryMetrics, SingleBatteryInfo};

    let design_cap = 50000;
    let full_cap = 45000;
    let remaining_cap = 30000;

    let health = (full_cap as f32 / design_cap as f32) * 100.0;
    let wear = 100.0 - health;
    let calc_pct = ((remaining_cap as f64 / full_cap as f64) * 100.0).round() as u8;

    let battery = BatteryMetrics {
        has_battery: true,
        is_charging: false,
        is_discharging: true,
        is_ac_connected: false,
        is_saver_active: false,
        is_critical: false,
        percentage: calc_pct,
        life_time_seconds: Some(7200), // 2 hours
        time_remaining_formatted: "2h 0m remaining".to_string(),
        remaining_capacity_mwh: remaining_cap,
        full_charge_capacity_mwh: full_cap,
        designed_capacity_mwh: design_cap,
        health_percent: Some(health),
        wear_percent: Some(wear),
        cycle_count: Some(150),
        rate_watts: Some(-15.0),
        voltage_volts: Some(11.8),
        temperature_c: Some(29.0),
        chemistry: "Lithium-Ion (Li-Ion)".to_string(),
        device_name: "L19C3PF5".to_string(),
        manufacturer: "SMP".to_string(),
        serial_number: "12345".to_string(),
        manufacture_date: Some("2023-05-10".to_string()),
        power_state_description: "On Battery (Discharging)".to_string(),
        batteries: vec![SingleBatteryInfo {
            name: "L19C3PF5".to_string(),
            manufacturer: "SMP".to_string(),
            serial_number: "12345".to_string(),
            chemistry: "Lithium-Ion (Li-Ion)".to_string(),
            manufacture_date: Some("2023-05-10".to_string()),
            designed_capacity_mwh: design_cap,
            full_charge_capacity_mwh: full_cap,
            current_capacity_mwh: remaining_cap,
            health_percent: Some(health),
            wear_percent: Some(wear),
            cycle_count: Some(150),
            voltage_volts: Some(11.8),
            rate_watts: Some(-15.0),
            temperature_c: Some(29.0),
            is_charging: false,
            is_discharging: true,
            is_critical: false,
        }],
    };

    assert!(battery.has_battery);
    assert_eq!(battery.percentage, 67);
    assert_eq!(battery.health_percent, Some(90.0));
    assert_eq!(battery.wear_percent, Some(10.0));
    assert_eq!(battery.cycle_count, Some(150));
    assert_eq!(battery.device_name, "L19C3PF5");
    assert_eq!(battery.chemistry, "Lithium-Ion (Li-Ion)");
    assert_eq!(battery.voltage_volts, Some(11.8));
    assert_eq!(battery.rate_watts, Some(-15.0));
    assert_eq!(battery.time_remaining_formatted, "2h 0m remaining");
}

#[test]
fn test_sensors_collector_with_battery() {
    use sidebar_native::telemetry::power::BatteryMetrics;
    use sidebar_native::telemetry::sensors::SensorsCollector;
    use sidebar_native::telemetry::TelemetrySnapshot;

    let snapshot = TelemetrySnapshot {
        battery: BatteryMetrics {
            has_battery: true,
            is_charging: true,
            is_discharging: false,
            is_ac_connected: true,
            is_saver_active: false,
            is_critical: false,
            percentage: 85,
            life_time_seconds: Some(1800),
            time_remaining_formatted: "30m until full".to_string(),
            remaining_capacity_mwh: 42500,
            full_charge_capacity_mwh: 50000,
            designed_capacity_mwh: 55000,
            health_percent: Some(90.9),
            wear_percent: Some(9.1),
            cycle_count: Some(88),
            rate_watts: Some(25.0),
            voltage_volts: Some(12.5),
            temperature_c: Some(31.0),
            chemistry: "Lithium-Ion (Li-Ion)".to_string(),
            device_name: "L20M3PC2".to_string(),
            manufacturer: "SMP".to_string(),
            serial_number: "4500".to_string(),
            manufacture_date: None,
            power_state_description: "Plugged in (Charging)".to_string(),
            batteries: vec![],
        },
        ..Default::default()
    };

    let cfg = AppConfig::default();
    let mut collector = SensorsCollector::new();
    let sensors = collector.collect(&snapshot, &cfg);

    let power_sensors: Vec<_> = sensors
        .iter()
        .filter(|s| s.category == "Power & Battery")
        .collect();

    assert!(!power_sensors.is_empty());
    assert!(power_sensors
        .iter()
        .any(|s| s.name.contains("Charge Level") && s.value.contains("85%")));
    assert!(power_sensors
        .iter()
        .any(|s| s.name.contains("Power Flow Rate") && s.value.contains("+25.00 W")));
    assert!(power_sensors
        .iter()
        .any(|s| s.name.contains("Terminal Voltage") && s.value.contains("12.500 V")));
    assert!(power_sensors
        .iter()
        .any(|s| s.name.contains("Energy Stored") && s.value.contains("42.5 Wh / 50.0 Wh")));
    assert!(power_sensors
        .iter()
        .any(|s| s.name.contains("Health & Degradation") && s.value.contains("90.9%")));
    assert!(power_sensors
        .iter()
        .any(|s| s.name.contains("Charge Cycle Count") && s.value.contains("88 Cycles")));
    assert!(power_sensors
        .iter()
        .any(|s| s.name.contains("Cell Temperature") && s.value.contains("31 °C")));
}

#[test]
fn test_process_network_ranking_and_stats() {
    let mut net_procs = [
        ProcessInfo {
            name: "idle_server".to_string(),
            tcp_sockets: 5,
            tcp_established: 0,
            tcp_listening: 5,
            udp_sockets: 0,
            active_sockets: 5,
            net_rx_bytes_sec: 0,
            net_tx_bytes_sec: 0,
            net_total_bytes_sec: 0,
            disk_total_bytes_sec: 0,
            cpu_usage: 0.1,
            ..Default::default()
        },
        ProcessInfo {
            name: "downloader".to_string(),
            tcp_sockets: 8,
            tcp_established: 8,
            tcp_listening: 0,
            udp_sockets: 0,
            active_sockets: 8,
            net_rx_bytes_sec: 1024 * 1024 * 5, // 5 MB/s
            net_tx_bytes_sec: 1024 * 100,      // 100 KB/s
            net_total_bytes_sec: 1024 * 1024 * 5 + 1024 * 100,
            disk_total_bytes_sec: 1024 * 500,
            cpu_usage: 5.0,
            ..Default::default()
        },
        ProcessInfo {
            name: "browser_tab".to_string(),
            tcp_sockets: 12,
            tcp_established: 4,
            tcp_listening: 0,
            udp_sockets: 2,
            active_sockets: 14,
            net_rx_bytes_sec: 0,
            net_tx_bytes_sec: 0,
            net_total_bytes_sec: 0,
            disk_total_bytes_sec: 100,
            cpu_usage: 1.0,
            ..Default::default()
        },
    ];

    net_procs.sort_by(|a, b| {
        let a_net = a.net_total_bytes_sec;
        let b_net = b.net_total_bytes_sec;
        if a_net > 0 || b_net > 0 {
            return b_net.cmp(&a_net);
        }

        if a.tcp_established != b.tcp_established {
            return b.tcp_established.cmp(&a.tcp_established);
        }

        if a.active_sockets != b.active_sockets {
            return b.active_sockets.cmp(&a.active_sockets);
        }

        b.cpu_usage
            .partial_cmp(&a.cpu_usage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    assert_eq!(net_procs[0].name, "downloader"); // Active throughput > 0
    assert_eq!(net_procs[1].name, "browser_tab"); // 4 established connections
    assert_eq!(net_procs[2].name, "idle_server"); // 0 established (listening only)
}

#[test]
fn test_etw_collector_run() {
    let mut collector = EtwNetworkCollector::new();
    std::thread::sleep(std::time::Duration::from_millis(500));
    let rates = collector.sample_and_drain();
    println!("ETW rates count: {}", rates.len());
    for (pid, (rx, tx)) in rates.iter().take(5) {
        println!("PID {}: rx={}, tx={}", pid, rx, tx);
    }
}

#[test]
fn test_elevation_check() {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token = HANDLE::default();
        let open_res = OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token);
        println!("OpenProcessToken: {:?}", open_res);
        if open_res.is_ok() {
            let mut elevation = TOKEN_ELEVATION::default();
            let mut return_length = 0u32;
            let get_res = GetTokenInformation(
                token,
                TokenElevation,
                Some(&mut elevation as *mut _ as *mut _),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut return_length,
            );
            let _ = CloseHandle(token);
            println!(
                "GetTokenInformation: {:?}, is_elevated: {}",
                get_res,
                elevation.TokenIsElevated != 0
            );
        }
    }
}
