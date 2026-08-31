#![allow(unused_imports, dead_code, unused_must_use)]

pub mod audio;
pub mod collector;
pub mod cpu;
pub mod gpu;
pub mod network;
pub mod power;
pub mod process;
pub mod ram;
pub mod sensors;
pub mod storage;
pub mod temperature;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use crate::config::AppConfig;
pub use audio::{AudioCollector, AudioMetrics};
pub use collector::TelemetryCollector;
pub use cpu::CpuMetrics;
pub use gpu::{GpuInfo, GpuMetrics};
pub use network::{NetworkAdapterInfo, NetworkMetrics};
pub use power::BatteryMetrics;
pub use process::{ProcessInfo, ProcessMemoryInfo};
pub use ram::RamMetrics;
pub use sensors::{HardwareSensor, SensorsCollector};
pub use storage::{DriveInfo, StorageMetrics};
pub use temperature::{TemperatureMetrics, TemperatureSensor};

#[derive(Debug, Clone, Default)]
pub struct TelemetrySnapshot {
    pub cpu: CpuMetrics,
    pub gpu: GpuMetrics,
    pub ram: RamMetrics,
    pub storage: StorageMetrics,
    pub network: NetworkMetrics,
    pub audio: AudioMetrics,
    pub battery: BatteryMetrics,
    pub temperature: TemperatureMetrics,
    pub all_sensors: Vec<HardwareSensor>,
    pub top_cpu_processes: Vec<ProcessInfo>,
    pub top_ram_processes: Vec<ProcessInfo>,
    pub top_disk_processes: Vec<ProcessInfo>,
    pub top_network_processes: Vec<ProcessInfo>,
    pub top_processes: Vec<ProcessInfo>,
    pub timestamp: String,
    pub machine_name: String,
    pub os_version: String,

    /// True when the ETW kernel-network session started successfully (requires admin).
    /// When false, per-process bandwidth (net_*_bytes_sec) will always be 0.
    pub etw_network_active: bool,

    // History
    pub cpu_history: Vec<f32>,
    pub ram_history: Vec<f32>,
}

pub struct TelemetryEngine {
    snapshot: Arc<RwLock<TelemetrySnapshot>>,
    running: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
}

impl TelemetryEngine {
    pub fn new() -> Self {
        Self {
            snapshot: Arc::new(RwLock::new(TelemetrySnapshot::default())),
            running: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn snapshot(&self) -> Arc<RwLock<TelemetrySnapshot>> {
        self.snapshot.clone()
    }

    pub fn set_paused(&self, paused: bool) {
        crate::log_debug!("Telemetry engine set_paused: {}", paused);
        self.paused.store(paused, Ordering::Relaxed);
    }

    pub fn start(&self, config: AppConfig) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }

        crate::log_info!("Starting background telemetry worker thread...");
        let snapshot = self.snapshot.clone();
        let running = self.running.clone();
        let paused = self.paused.clone();

        let machine_name =
            std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Windows PC".to_string());
        let os_version = "Windows 11 / 10".to_string();

        thread::spawn(move || {
            crate::log_info!("Telemetry worker thread running successfully.");

            // Register collectors following Dependency Inversion & Open-Closed Principles
            let mut collectors: Vec<Box<dyn TelemetryCollector>> = vec![
                Box::new(cpu::CpuCollector::new()),
                Box::new(gpu::GpuCollector::new()),
                Box::new(ram::RamCollector::new()),
                Box::new(storage::StorageCollector::new()),
                Box::new(network::NetworkCollector::new()),
                Box::new(audio::AudioCollector::new()),
                Box::new(power::PowerCollector::new()),
                Box::new(temperature::TemperatureCollector::new()),
                Box::new(process::ProcessCollector::new()),
                Box::new(sensors::SensorsCollector::new()),
            ];

            let mut cpu_hist: Vec<f32> = Vec::with_capacity(30);
            let mut ram_hist: Vec<f32> = Vec::with_capacity(30);
            let mut sample_count: u64 = 0;

            while running.load(Ordering::Relaxed) {
                if !paused.load(Ordering::Relaxed) {
                    let start_cycle = Instant::now();
                    let mut working_snapshot = TelemetrySnapshot::default();

                    for collector in collectors.iter_mut() {
                        collector.update(&mut working_snapshot, &config);
                    }

                    working_snapshot.machine_name = machine_name.clone();
                    working_snapshot.os_version = os_version.clone();
                    working_snapshot.timestamp = chrono_or_simple_time();

                    // Update history buffers
                    if cpu_hist.len() >= 30 {
                        cpu_hist.remove(0);
                    }
                    cpu_hist.push(working_snapshot.cpu.global_usage);
                    working_snapshot.cpu_history = cpu_hist.clone();

                    if ram_hist.len() >= 30 {
                        ram_hist.remove(0);
                    }
                    ram_hist.push(working_snapshot.ram.usage_percentage);
                    working_snapshot.ram_history = ram_hist.clone();

                    let total_ms = start_cycle.elapsed().as_secs_f64() * 1000.0;
                    sample_count += 1;
                    if sample_count % 10 == 1 {
                        crate::log_debug!(
                            "Telemetry cycle #{}: executed {} collectors in {:.2}ms",
                            sample_count,
                            collectors.len(),
                            total_ms
                        );
                    }

                    if let Ok(mut lock) = snapshot.write() {
                        *lock = working_snapshot;
                    }
                }

                let interval = if paused.load(Ordering::Relaxed) {
                    Duration::from_millis(2000)
                } else {
                    Duration::from_millis(config.poll_interval_ms.max(500))
                };

                thread::sleep(interval);
            }

            crate::log_info!("Telemetry worker thread terminated.");
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

fn chrono_or_simple_time() -> String {
    use windows::Win32::System::SystemInformation::GetLocalTime;
    unsafe {
        let st = GetLocalTime();
        format!("{:02}:{:02}:{:02}", st.wHour, st.wMinute, st.wSecond)
    }
}
