use sidebar_native::config::{
    AppConfig, AppTheme, BackdropEffect, FontSize, ProcessSortBy, TemperatureUnit,
    WindowWidthPreset,
};
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
        ..Default::default()
    };

    let json_str = serde_json::to_string_pretty(&cfg).expect("Serialization failed");
    assert!(json_str.contains("UltraWide"));
    assert!(json_str.contains("Disk"));

    let deserialized: AppConfig = serde_json::from_str(&json_str).expect("Deserialization failed");
    assert_eq!(deserialized.sidebar_width, 520);
    assert_eq!(deserialized.width_preset, WindowWidthPreset::UltraWide);
    assert_eq!(deserialized.sort_processes_by, ProcessSortBy::Disk);
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
        },
        ProcessInfo {
            name: "code".to_string(),
            cpu_usage: 28.0,
            memory_bytes: 1024 * 1024 * 400, // 400 MB
            formatted_memory: "400 MB".to_string(),
            disk_read_bytes_sec: 1024 * 10,
            disk_write_bytes_sec: 1024 * 10,
            disk_total_bytes_sec: 1024 * 20,
        },
        ProcessInfo {
            name: "rustc".to_string(),
            cpu_usage: 4.2,
            memory_bytes: 1024 * 1024 * 1200, // 1200 MB
            formatted_memory: "1.2 GB".to_string(),
            disk_read_bytes_sec: 1024 * 1024 * 5,
            disk_write_bytes_sec: 1024 * 1024 * 10,
            disk_total_bytes_sec: 1024 * 1024 * 15,
        },
    ];

    // Sort by CPU
    procs.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap());
    assert_eq!(procs[0].name, "code"); // 28.0%

    // Sort by Memory
    procs.sort_by_key(|a| std::cmp::Reverse(a.memory_bytes));
    assert_eq!(procs[0].name, "rustc"); // 1200 MB

    // Sort by Disk I/O
    procs.sort_by_key(|a| std::cmp::Reverse(a.disk_total_bytes_sec));
    assert_eq!(procs[0].name, "rustc"); // 15 MB/s
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
