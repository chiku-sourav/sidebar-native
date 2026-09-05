pub mod audio;
pub mod battery;
pub mod cpu;
pub mod gpu;
pub mod header;
pub mod network;
pub mod processes;
pub mod ram;
pub mod sensors;
pub mod storage;
pub mod system;
pub mod virtual_memory;
pub mod welcome;

use super::context::RenderContext;
use crate::config::AppConfig;
use crate::telemetry::TelemetrySnapshot;
use windows::Win32::Graphics::Gdi::HDC;

/// Single-responsibility contract for UI card components following SOLID principles (SRP, OCP, LSP, DIP).
pub trait CardRenderer: Send + Sync {
    /// Human-readable card identifier
    fn name(&self) -> &'static str;

    /// Determines whether this card is enabled and visible based on current configuration
    fn is_enabled(&self, config: &AppConfig) -> bool;

    /// Calculates exact height required for rendering this card in pixels
    fn calculate_height(&self, snapshot: &TelemetrySnapshot, config: &AppConfig) -> i32;

    /// Renders the card into the double-buffered device context
    fn render(
        &self,
        ctx: &RenderContext,
        hdc: HDC,
        x: i32,
        y: i32,
        w: i32,
        snapshot: &TelemetrySnapshot,
        config: &AppConfig,
    );
}
