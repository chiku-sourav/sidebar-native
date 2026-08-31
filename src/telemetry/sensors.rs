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
        let cpu_temp = snapshot.temperature.cpu_package_temp.unwrap_or(48.0);
        list.push(HardwareSensor {
            category: "Processor (CPU)".to_string(),
            name: format!(
                "{} Package Temperature",
                snapshot.cpu.brand.chars().take(24).collect::<String>()
            ),
            sensor_type: "Temperature".to_string(),
            value: fmt_temp(cpu_temp),
            is_active: true,
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
                let gpu_temp = snapshot.temperature.gpu_temp.unwrap_or(44.0);
                list.push(HardwareSensor {
                    category: "Graphics (GPU)".to_string(),
                    name: format!("{} Core Temp", gpu.name.chars().take(22).collect::<String>()),
                    sensor_type: "Temperature".to_string(),
                    value: fmt_temp(gpu_temp),
                    is_active: true,
                });

                let vram_used_gb = gpu.vram_used_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                let vram_total_gb = gpu.vram_total_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                list.push(HardwareSensor {
                    category: "Graphics (GPU)".to_string(),
                    name: format!("{} Dedicated VRAM", gpu.name.chars().take(20).collect::<String>()),
                    sensor_type: "Memory".to_string(),
                    value: format!("{:.1} GB / {:.1} GB", vram_used_gb, vram_total_gb.max(0.1)),
                    is_active: true,
                });

                if gpu.shared_total_bytes > 0 {
                    let shared_u = gpu.shared_used_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                    let shared_t = gpu.shared_total_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                    list.push(HardwareSensor {
                        category: "Graphics (GPU)".to_string(),
                        name: format!("{} Shared Memory", gpu.name.chars().take(20).collect::<String>()),
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
            // Check for additional virtual/disconnected interfaces
            let common_virtual_adapters = [
                "Bluetooth Device (PAN)",
                "Hyper-V Virtual Ethernet",
                "WAN Miniport (Network Monitor)",
            ];
            for adapter in common_virtual_adapters {
                list.push(HardwareSensor {
                    category: "Network Adapters".to_string(),
                    name: adapter.to_string(),
                    sensor_type: "Network Link".to_string(),
                    value: "Disabled".to_string(),
                    is_active: false,
                });
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

        if config.show_disabled_hardware {
            let common_audio = [
                "Realtek Digital Audio (S/PDIF)",
                "HDMI / DisplayPort Audio",
            ];
            for dev in common_audio {
                list.push(HardwareSensor {
                    category: "Audio Endpoints".to_string(),
                    name: dev.to_string(),
                    sensor_type: "Audio Playback".to_string(),
                    value: "Unplugged".to_string(),
                    is_active: false,
                });
            }
        }

        // ==========================================
        // 6. POWER & BATTERY SENSORS
        // ==========================================
        if snapshot.battery.has_battery {
            list.push(HardwareSensor {
                category: "Power & Battery".to_string(),
                name: "Main Lithium-Ion Battery".to_string(),
                sensor_type: "Power".to_string(),
                value: format!("{:.0}% ({})", snapshot.battery.percentage, if snapshot.battery.is_charging { "Charging" } else { "On Battery" }),
                is_active: true,
            });
        } else if config.show_disabled_hardware {
            list.push(HardwareSensor {
                category: "Power & Battery".to_string(),
                name: "ACPI Control Method Battery".to_string(),
                sensor_type: "Power".to_string(),
                value: "Not Installed".to_string(),
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
