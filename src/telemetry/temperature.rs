use sysinfo::Components;
use windows::core::{s, w};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

use super::nvml::NvmlHelper;
use super::pdh::{PdhHCounter, PdhHelper};

#[derive(Debug, Clone, Default)]
pub struct TemperatureSensor {
    pub label: String,
    pub temperature_c: f32,
    pub max_c: Option<f32>,
    pub critical_c: Option<f32>,
}

#[derive(Debug, Clone, Default)]
pub struct TemperatureMetrics {
    pub cpu_package_temp: Option<f32>,
    pub gpu_temp: Option<f32>,
    pub sensors: Vec<TemperatureSensor>,
}

pub struct TemperatureCollector {
    components: Components,
    pdh: Option<PdhHelper>,
    h_thermal_prec: PdhHCounter,
    h_thermal_temp: PdhHCounter,
    nvml: Option<NvmlHelper>,
}

unsafe impl Send for TemperatureCollector {}
unsafe impl Sync for TemperatureCollector {}

impl TemperatureCollector {
    pub fn new() -> Self {
        let components = Components::new_with_refreshed_list();
        let mut pdh = PdhHelper::new();
        let mut h_thermal_prec = 0;
        let mut h_thermal_temp = 0;
        if let Some(p) = pdh.as_mut() {
            h_thermal_prec =
                p.add_counter("\\Thermal Zone Information(*)\\High Precision Temperature");
            h_thermal_temp = p.add_counter("\\Thermal Zone Information(*)\\Temperature");
        }
        let nvml = NvmlHelper::new();

        Self {
            components,
            pdh,
            h_thermal_prec,
            h_thermal_temp,
            nvml,
        }
    }

    pub fn collect(&mut self) -> TemperatureMetrics {
        let mut sensors = Vec::new();
        let mut cpu_temp: Option<f32> = None;
        let mut gpu_temp: Option<f32> = None;

        // 1. Query real hardware temperatures from NVML (NVIDIA GPUs)
        if let Some(nvml) = &self.nvml {
            let nv_gpus = nvml.query_gpus();
            for (name, temp_c, _gen, _width) in nv_gpus {
                if temp_c > 0.0 && temp_c < 125.0 {
                    if gpu_temp.is_none() || temp_c > gpu_temp.unwrap_or(0.0) {
                        gpu_temp = Some(temp_c);
                    }
                    sensors.push(TemperatureSensor {
                        label: format!("{} Core", name),
                        temperature_c: temp_c,
                        max_c: None,
                        critical_c: None,
                    });
                }
            }
        }

        // 2. Query Windows ACPI Thermal Zones from PDH (CPU / Motherboard package thermals)
        if let Some(p) = self.pdh.as_mut() {
            if p.collect() {
                // Try High Precision Temperature first (tenths of Kelvin: K * 10)
                let prec_items = p.read_array(self.h_thermal_prec);
                if !prec_items.is_empty() {
                    for item in prec_items {
                        if item.value > 1000.0 && item.value < 4500.0 {
                            let temp_c = ((item.value / 10.0) - 273.15) as f32;
                            if temp_c > 0.0 && temp_c < 130.0 {
                                let label = clean_thermal_zone_label(&item.name);
                                if cpu_temp.is_none() || temp_c > cpu_temp.unwrap_or(0.0) {
                                    cpu_temp = Some(temp_c);
                                }
                                sensors.push(TemperatureSensor {
                                    label,
                                    temperature_c: temp_c,
                                    max_c: None,
                                    critical_c: None,
                                });
                            }
                        }
                    }
                } else {
                    // Fall back to standard Temperature (Kelvin)
                    let temp_items = p.read_array(self.h_thermal_temp);
                    for item in temp_items {
                        if item.value > 100.0 && item.value < 450.0 {
                            let temp_c = (item.value - 273.15) as f32;
                            if temp_c > 0.0 && temp_c < 130.0 {
                                let label = clean_thermal_zone_label(&item.name);
                                if cpu_temp.is_none() || temp_c > cpu_temp.unwrap_or(0.0) {
                                    cpu_temp = Some(temp_c);
                                }
                                sensors.push(TemperatureSensor {
                                    label,
                                    temperature_c: temp_c,
                                    max_c: None,
                                    critical_c: None,
                                });
                            }
                        }
                    }
                }
            }
        }

        // 3. Query sysinfo components for any hardware/platform thermal sensors
        self.components.refresh(true);
        for comp in &self.components {
            let label = comp.label().trim().to_string();
            let temp_val = comp.temperature().unwrap_or(0.0);
            let max_c = comp.max();
            let crit_c = comp.critical();

            if temp_val > 0.0 && temp_val < 130.0 {
                let lcase = label.to_lowercase();
                if lcase.contains("cpu") || lcase.contains("core") || lcase.contains("package") {
                    if cpu_temp.is_none() || temp_val > cpu_temp.unwrap_or(0.0) {
                        cpu_temp = Some(temp_val);
                    }
                }

                if lcase.contains("gpu") {
                    if gpu_temp.is_none() || temp_val > gpu_temp.unwrap_or(0.0) {
                        gpu_temp = Some(temp_val);
                    }
                }

                if !sensors.iter().any(|s| s.label.eq_ignore_ascii_case(&label)) {
                    sensors.push(TemperatureSensor {
                        label,
                        temperature_c: temp_val,
                        max_c,
                        critical_c: crit_c,
                    });
                }
            }
        }

        // 4. NOTE: If no sensors returned valid values, cpu_package_temp and gpu_temp
        // remain None. We do not inject fake constants (e.g. 48.0 or 44.0).
        TemperatureMetrics {
            cpu_package_temp: cpu_temp,
            gpu_temp,
            sensors,
        }
    }
}

fn clean_thermal_zone_label(name: &str) -> String {
    let clean = name.trim_start_matches('\\').trim_start_matches('_');
    if clean.is_empty() {
        "ACPI Thermal Zone".to_string()
    } else {
        format!("ACPI Thermal Zone ({})", clean)
    }
}

impl super::collector::TelemetryCollector for TemperatureCollector {
    fn name(&self) -> &'static str {
        "Thermals"
    }

    fn update(
        &mut self,
        snapshot: &mut super::TelemetrySnapshot,
        _config: &crate::config::AppConfig,
    ) {
        snapshot.temperature = self.collect();
    }
}
