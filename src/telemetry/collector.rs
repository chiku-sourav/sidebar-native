use crate::config::AppConfig;
use crate::telemetry::TelemetrySnapshot;

/// Abstraction for hardware and system telemetry collectors following SOLID principles (OCP, LSP, DIP).
pub trait TelemetryCollector: Send + Sync {
    /// Human-readable identifier for performance logging and debugging
    fn name(&self) -> &'static str;

    /// Collects metrics and updates the corresponding section of TelemetrySnapshot
    fn update(&mut self, snapshot: &mut TelemetrySnapshot, config: &AppConfig);
}
