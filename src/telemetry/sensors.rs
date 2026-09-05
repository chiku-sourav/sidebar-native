#![allow(unused_imports, dead_code, unused_must_use)]

use crate::config::{AppConfig, TemperatureUnit};
use crate::telemetry::collector::TelemetryCollector;
use crate::telemetry::TelemetrySnapshot;

#[derive(Debug, Clone, Default)]
pub struct HardwareSensor {
    pub category: String,
    pub name: String,
    pub sensor_type: String,
    pub value: String,
    pub is_active: bool,
}

pub struct SensorsCollector;

impl SensorsCollector {
    pub fn new() -> Self {
        Self
    }

    pub fn collect(
        &mut self,
        snapshot: &TelemetrySnapshot,
        config: &AppConfig,
    ) -> Vec<HardwareSensor> {
        let mut list = Vec::new();

        let fmt_temp = |c: f32| -> String {
            match config.temperature_unit {
                TemperatureUnit::Celsius => format!("{:.0} °C", c),
                TemperatureUnit::Fahrenheit => format!("{:.0} °F", (c * 9.0 / 5.0) + 32.0),
            }
        };

        // ==========================================
        // 1. CPU & THERMAL ZONE SENSORS
        // ==========================================
        let cpu_temp_val = snapshot
            .temperature
            .cpu_package_temp
            .map(&fmt_temp)
            .unwrap_or_else(|| "N/A".to_string());
        list.push(HardwareSensor {
            category: "Processor (CPU)".to_string(),
            name: format!(
                "{} Package Temperature",
                snapshot.cpu.brand.chars().take(24).collect::<String>()
            ),
            sensor_type: "Temperature".to_string(),
            value: cpu_temp_val,
            is_active: snapshot.temperature.cpu_package_temp.is_some(),
        });

        if snapshot.cpu.frequency_mhz > 0 {
            let freq_val = if config.use_ghz {
                format!("{:.2} GHz", snapshot.cpu.frequency_mhz as f64 / 1000.0)
            } else {
                format!("{} MHz", snapshot.cpu.frequency_mhz)
            };
            list.push(HardwareSensor {
                category: "Processor (CPU)".to_string(),
                name: "CPU Clock Frequency".to_string(),
                sensor_type: "Clock Speed".to_string(),
                value: freq_val,
                is_active: true,
            });
        }

        list.push(HardwareSensor {
            category: "Processor (CPU)".to_string(),
            name: "CPU Global Utilization Load".to_string(),
            sensor_type: "Load".to_string(),
            value: format!("{:.1}%", snapshot.cpu.global_usage),
            is_active: true,
        });

        // Add additional discovered thermal zones from sysinfo
        for (i, sensor) in snapshot.temperature.sensors.iter().enumerate() {
            let label = if sensor.label.is_empty() {
                format!("ACPI Thermal Zone #{}", i + 1)
            } else {
                sensor.label.clone()
            };
            list.push(HardwareSensor {
                category: "Motherboard & ACPI".to_string(),
                name: label,
                sensor_type: "Temperature".to_string(),
                value: fmt_temp(sensor.temperature_c),
                is_active: true,
            });
        }

        // ==========================================
        // 2. GRAPHICS (GPU) & ADAPTER SENSORS
        // ==========================================
        for gpu in &snapshot.gpu.gpus {
            let active = gpu.is_active;

            if active {
                let gpu_temp = gpu.temperature_c.or(snapshot.temperature.gpu_temp);
                let gpu_temp_val = gpu_temp.map(&fmt_temp).unwrap_or_else(|| "N/A".to_string());
                list.push(HardwareSensor {
                    category: "Graphics (GPU)".to_string(),
                    name: format!(
                        "{} Core Temp",
                        gpu.name.chars().take(22).collect::<String>()
                    ),
                    sensor_type: "Temperature".to_string(),
                    value: gpu_temp_val,
                    is_active: gpu_temp.is_some(),
                });

                let vram_used_gb = gpu.vram_used_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                let vram_total_gb = gpu.vram_total_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                list.push(HardwareSensor {
                    category: "Graphics (GPU)".to_string(),
                    name: format!(
                        "{} Dedicated VRAM",
                        gpu.name.chars().take(20).collect::<String>()
                    ),
                    sensor_type: "Memory".to_string(),
                    value: format!("{:.1} GB / {:.1} GB", vram_used_gb, vram_total_gb.max(0.1)),
                    is_active: true,
                });

                if gpu.shared_total_bytes > 0 {
                    let shared_u = gpu.shared_used_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                    let shared_t = gpu.shared_total_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                    list.push(HardwareSensor {
                        category: "Graphics (GPU)".to_string(),
                        name: format!(
                            "{} Shared Memory",
                            gpu.name.chars().take(20).collect::<String>()
                        ),
                        sensor_type: "Memory".to_string(),
                        value: format!("{:.1} GB / {:.1} GB", shared_u, shared_t.max(0.1)),
                        is_active: true,
                    });
                }
            } else if config.show_disabled_hardware {
                list.push(HardwareSensor {
                    category: "Graphics (GPU)".to_string(),
                    name: format!("{} (dGPU)", gpu.name.chars().take(22).collect::<String>()),
                    sensor_type: "Display Adapter".to_string(),
                    value: "Standby (D3Cold)".to_string(),
                    is_active: false,
                });
            }
        }

        // ==========================================
        // 3. STORAGE DRIVES & SENSORS
        // ==========================================
        for drive in &snapshot.storage.drives {
            let free_gb = drive.free_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
            let tot_gb = drive.total_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
            list.push(HardwareSensor {
                category: "Storage & Disks".to_string(),
                name: format!("Volume ({}) Physical Capacity", drive.letter),
                sensor_type: "Storage".to_string(),
                value: format!("{:.1} GB Free / {:.1} GB", free_gb, tot_gb),
                is_active: true,
            });
        }

        // If show_disabled_hardware is enabled, discover optical / unmounted letters
        if config.show_disabled_hardware {
            let active_letters: Vec<char> = snapshot
                .storage
                .drives
                .iter()
                .filter_map(|d| d.letter.chars().next())
                .collect();

            unsafe {
                let drive_mask = windows::Win32::Storage::FileSystem::GetLogicalDrives();
                for i in 0..26 {
                    let letter = (b'A' + i as u8) as char;
                    if (drive_mask & (1 << i)) != 0 && !active_letters.contains(&letter) {
                        list.push(HardwareSensor {
                            category: "Storage & Disks".to_string(),
                            name: format!("Drive ({}:) Removable / Optical", letter),
                            sensor_type: "Storage".to_string(),
                            value: "No Media".to_string(),
                            is_active: false,
                        });
                    }
                }
            }
        }

        // ==========================================
        // 4. NETWORK ADAPTERS & SENSORS
        // ==========================================
        let net_has_ip =
            !snapshot.network.local_ip.is_empty() && snapshot.network.local_ip != "127.0.0.1";
        list.push(HardwareSensor {
            category: "Network Adapters".to_string(),
            name: snapshot.network.primary_interface.clone(),
            sensor_type: "Network Link".to_string(),
            value: if net_has_ip {
                format!("Active ({})", snapshot.network.local_ip)
            } else {
                "Connected (DHCP)".to_string()
            },
            is_active: true,
        });

        if config.show_disabled_hardware {
            for adapter in &snapshot.network.adapters {
                if !adapter.is_up {
                    list.push(HardwareSensor {
                        category: "Network Adapters".to_string(),
                        name: adapter.display_name.clone(),
                        sensor_type: "Network Link".to_string(),
                        value: "Disconnected".to_string(),
                        is_active: false,
                    });
                }
            }
        }

        // ==========================================
        // 5. AUDIO PLAYBACK HARDWARE
        // ==========================================
        list.push(HardwareSensor {
            category: "Audio Endpoints".to_string(),
            name: snapshot.audio.device_name.clone(),
            sensor_type: "Audio Playback".to_string(),
            value: if snapshot.audio.is_muted {
                "Muted (0%)".to_string()
            } else {
                format!("Active (Volume: {:.0}%)", snapshot.audio.volume_percent)
            },
            is_active: true,
        });

        // ==========================================
        // 6. POWER & BATTERY SENSORS
        // ==========================================
        if snapshot.battery.has_battery {
            let bat_name = if !snapshot.battery.device_name.is_empty() {
                if !snapshot.battery.manufacturer.is_empty() {
                    format!(
                        "{} {}",
                        snapshot.battery.manufacturer, snapshot.battery.device_name
                    )
                } else {
                    snapshot.battery.device_name.clone()
                }
            } else {
                "Main Battery".to_string()
            };

            list.push(HardwareSensor {
                category: "Power & Battery".to_string(),
                name: format!("{} Charge Level", bat_name),
                sensor_type: "Charge".to_string(),
                value: format!(
                    "{}% ({})",
                    snapshot.battery.percentage, snapshot.battery.power_state_description
                ),
                is_active: true,
            });

            if let Some(rate) = snapshot.battery.rate_watts {
                list.push(HardwareSensor {
                    category: "Power & Battery".to_string(),
                    name: "Battery Power Flow Rate".to_string(),
                    sensor_type: "Power".to_string(),
                    value: if rate > 0.0 {
                        format!("+{:.2} W (Charging)", rate)
                    } else if rate < 0.0 {
                        format!("{:.2} W (Discharging)", rate)
                    } else {
                        "0.00 W (Idle)".to_string()
                    },
                    is_active: true,
                });
            }

            if let Some(voltage) = snapshot.battery.voltage_volts {
                list.push(HardwareSensor {
                    category: "Power & Battery".to_string(),
                    name: "Battery Terminal Voltage".to_string(),
                    sensor_type: "Voltage".to_string(),
                    value: format!("{:.3} V", voltage),
                    is_active: true,
                });
            }

            if snapshot.battery.full_charge_capacity_mwh > 0 {
                let rem_wh = snapshot.battery.remaining_capacity_mwh as f32 / 1000.0;
                let full_wh = snapshot.battery.full_charge_capacity_mwh as f32 / 1000.0;
                list.push(HardwareSensor {
                    category: "Power & Battery".to_string(),
                    name: "Battery Energy Stored".to_string(),
                    sensor_type: "Energy".to_string(),
                    value: format!("{:.1} Wh / {:.1} Wh", rem_wh, full_wh),
                    is_active: true,
                });
            }

            if let Some(health) = snapshot.battery.health_percent {
                let wear_str = snapshot
                    .battery
                    .wear_percent
                    .map(|w| format!(" ({:.1}% Wear)", w))
                    .unwrap_or_default();
                list.push(HardwareSensor {
                    category: "Power & Battery".to_string(),
                    name: "Battery Health & Degradation".to_string(),
                    sensor_type: "Health".to_string(),
                    value: format!("{:.1}%{}", health, wear_str),
                    is_active: true,
                });
            }

            if let Some(cycles) = snapshot.battery.cycle_count {
                list.push(HardwareSensor {
                    category: "Power & Battery".to_string(),
                    name: "Battery Charge Cycle Count".to_string(),
                    sensor_type: "Cycles".to_string(),
                    value: format!("{} Cycles", cycles),
                    is_active: true,
                });
            }

            if let Some(temp_c) = snapshot.battery.temperature_c {
                list.push(HardwareSensor {
                    category: "Power & Battery".to_string(),
                    name: "Battery Cell Temperature".to_string(),
                    sensor_type: "Temperature".to_string(),
                    value: fmt_temp(temp_c),
                    is_active: true,
                });
            }
        } else if config.show_disabled_hardware {
            list.push(HardwareSensor {
                category: "Power & Battery".to_string(),
                name: "ACPI Control Method Battery".to_string(),
                sensor_type: "Power".to_string(),
                value: "Not Installed (AC Power)".to_string(),
                is_active: false,
            });
        }

        // ==========================================
        // 7. MEMORY & VIRTUAL MEMORY SENSORS
        // ==========================================
        list.push(HardwareSensor {
            category: "System Memory".to_string(),
            name: "Physical RAM Allocation".to_string(),
            sensor_type: "Memory".to_string(),
            value: format!(
                "{:.1} GB / {:.1} GB ({:.0}%)",
                snapshot.ram.used_bytes as f32 / (1024.0 * 1024.0 * 1024.0),
                snapshot.ram.total_bytes as f32 / (1024.0 * 1024.0 * 1024.0),
                snapshot.ram.usage_percentage
            ),
            is_active: true,
        });

        list.push(HardwareSensor {
            category: "System Memory".to_string(),
            name: "Page File Commitment".to_string(),
            sensor_type: "Virtual Memory".to_string(),
            value: format!(
                "{:.1} GB ({:.1}%)",
                snapshot.ram.committed_bytes as f32 / (1024.0 * 1024.0 * 1024.0),
                snapshot.ram.page_file_usage_pct
            ),
            is_active: true,
        });

        // ==========================================
        // 8. ADVANCED HARDWARE & FIRMWARE SENSORS (gated by config.adv_sensors)
        // ==========================================
        if config.adv_sensors {
            // CPU Advanced
            list.push(HardwareSensor {
                category: "Processor (CPU)".to_string(),
                name: "CPU Topology".to_string(),
                sensor_type: "Topology".to_string(),
                value: format!(
                    "{} Phys • {} Log • {} Socket",
                    snapshot.cpu.physical_core_count,
                    snapshot.cpu.core_count,
                    snapshot.cpu.socket_count
                ),
                is_active: true,
            });
            list.push(HardwareSensor {
                category: "Processor (CPU)".to_string(),
                name: "Base / Boost Clock".to_string(),
                sensor_type: "Clock Speed".to_string(),
                value: format!(
                    "{:.2} GHz / {:.2} GHz",
                    snapshot.cpu.base_clock_mhz as f64 / 1000.0,
                    snapshot.cpu.boost_clock_mhz as f64 / 1000.0
                ),
                is_active: true,
            });
            list.push(HardwareSensor {
                category: "Processor (CPU)".to_string(),
                name: "Context Switches & IRQs".to_string(),
                sensor_type: "System Rate".to_string(),
                value: format!(
                    "{}/s Ctx • {}/s IRQ",
                    snapshot.cpu.context_switches_per_sec, snapshot.cpu.interrupts_per_sec
                ),
                is_active: true,
            });

            // RAM Advanced
            list.push(HardwareSensor {
                category: "System Memory".to_string(),
                name: "Hardware Reserved Memory".to_string(),
                sensor_type: "Memory".to_string(),
                value: format!(
                    "{:.0} MB",
                    snapshot.ram.hardware_reserved_bytes as f32 / (1024.0 * 1024.0)
                ),
                is_active: true,
            });
            list.push(HardwareSensor {
                category: "System Memory".to_string(),
                name: "Non-Paged Kernel Pool".to_string(),
                sensor_type: "Memory".to_string(),
                value: format!(
                    "{:.0} MB",
                    snapshot.ram.nonpaged_pool_bytes as f32 / (1024.0 * 1024.0)
                ),
                is_active: true,
            });
            list.push(HardwareSensor {
                category: "System Memory".to_string(),
                name: "Paged Kernel Pool".to_string(),
                sensor_type: "Memory".to_string(),
                value: format!(
                    "{:.1} GB",
                    snapshot.ram.paged_pool_bytes as f32 / (1024.0 * 1024.0 * 1024.0)
                ),
                is_active: true,
            });
            list.push(HardwareSensor {
                category: "System Memory".to_string(),
                name: "Standby Cache / Modified".to_string(),
                sensor_type: "Memory".to_string(),
                value: format!(
                    "{:.1} GB / {:.0} MB",
                    snapshot.ram.standby_bytes as f32 / (1024.0 * 1024.0 * 1024.0),
                    snapshot.ram.modified_bytes as f32 / (1024.0 * 1024.0)
                ),
                is_active: true,
            });
            list.push(HardwareSensor {
                category: "System Memory".to_string(),
                name: "System File Cache".to_string(),
                sensor_type: "Memory".to_string(),
                value: format!(
                    "{:.1} GB",
                    snapshot.ram.system_cache_bytes as f32 / (1024.0 * 1024.0 * 1024.0)
                ),
                is_active: true,
            });

            // GPU Advanced
            for gpu in &snapshot.gpu.gpus {
                if gpu.is_active {
                    list.push(HardwareSensor {
                        category: "Graphics (GPU)".to_string(),
                        name: format!(
                            "{} 3D & Copy Engine",
                            gpu.name.chars().take(20).collect::<String>()
                        ),
                        sensor_type: "Load".to_string(),
                        value: format!(
                            "3D: {:.0}% • Copy: {:.0}%",
                            gpu.gpu_usage_pct, gpu.copy_engine_pct
                        ),
                        is_active: true,
                    });
                    list.push(HardwareSensor {
                        category: "Graphics (GPU)".to_string(),
                        name: format!(
                            "{} Video Encode/Decode",
                            gpu.name.chars().take(20).collect::<String>()
                        ),
                        sensor_type: "Load".to_string(),
                        value: format!(
                            "Enc: {:.0}% • Dec: {:.0}%",
                            gpu.video_encode_pct, gpu.video_decode_pct
                        ),
                        is_active: true,
                    });
                    if !gpu.driver_version.is_empty() {
                        list.push(HardwareSensor {
                            category: "Graphics (GPU)".to_string(),
                            name: format!(
                                "{} Driver Version",
                                gpu.name.chars().take(20).collect::<String>()
                            ),
                            sensor_type: "Driver".to_string(),
                            value: format!("v{}", gpu.driver_version),
                            is_active: true,
                        });
                    }
                }
            }

            // Storage Advanced
            for drive in &snapshot.storage.drives {
                list.push(HardwareSensor {
                    category: "Storage & Disks".to_string(),
                    name: format!("Drive ({}) IOPS Rate", drive.letter),
                    sensor_type: "I/O Rate".to_string(),
                    value: format!("R: {} IOPS • W: {} IOPS", drive.iops_read, drive.iops_write),
                    is_active: true,
                });
                list.push(HardwareSensor {
                    category: "Storage & Disks".to_string(),
                    name: format!("Drive ({}) Latency & Queue", drive.letter),
                    sensor_type: "Latency".to_string(),
                    value: format!(
                        "{:.1}ms R / {:.1}ms W • Q: {:.1}",
                        drive.read_latency_ms, drive.write_latency_ms, drive.queue_depth
                    ),
                    is_active: true,
                });
            }

            // Firmware & Platform Category
            if let Some(bios) = &snapshot.bios {
                list.push(HardwareSensor {
                    category: "Firmware & Platform".to_string(),
                    name: "BIOS Firmware".to_string(),
                    sensor_type: "Firmware".to_string(),
                    value: format!("{} v{} ({})", bios.vendor, bios.version, bios.release_date),
                    is_active: true,
                });
                list.push(HardwareSensor {
                    category: "Firmware & Platform".to_string(),
                    name: "UEFI Secure Boot".to_string(),
                    sensor_type: "Security".to_string(),
                    value: bios.secure_boot.clone(),
                    is_active: true,
                });
                let is_tpm_active =
                    bios.tpm_version != "Not Detected" && bios.tpm_version != "None";
                let tpm_val = if is_tpm_active {
                    format!("TPM {}", bios.tpm_version)
                } else {
                    "Not Detected".to_string()
                };
                list.push(HardwareSensor {
                    category: "Firmware & Platform".to_string(),
                    name: "Trusted Platform Module (TPM)".to_string(),
                    sensor_type: "Security".to_string(),
                    value: tpm_val,
                    is_active: is_tpm_active,
                });
                list.push(HardwareSensor {
                    category: "Firmware & Platform".to_string(),
                    name: "Motherboard / BaseBoard".to_string(),
                    sensor_type: "Mainboard".to_string(),
                    value: format!("{} {}", bios.motherboard_mfg, bios.motherboard_product),
                    is_active: true,
                });
            }
        }

        list
    }
}

impl TelemetryCollector for SensorsCollector {
    fn name(&self) -> &'static str {
        "Sensors Explorer"
    }

    fn update(&mut self, snapshot: &mut TelemetrySnapshot, config: &AppConfig) {
        snapshot.all_sensors = self.collect(snapshot, config);
    }
}
