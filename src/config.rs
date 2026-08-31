use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppTheme {
    Auto,      // Windows System Sync
    DarkSlate, // Windows 11 Slate Dark
    LightMode, // Windows 11 Clean Light
    OledBlack, // Pitch Black #000000
    Nord,      // Arctic Blue / Slate
    Cyberpunk, // Neon Cyan / Purple / Amber
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackdropEffect {
    None,
    Mica,
    Acrylic,
    MicaAlt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemperatureUnit {
    Celsius,
    Fahrenheit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DateFormat {
    Disabled,
    Short,  // MM/DD/YYYY
    Normal, // Mon, Jan 2
    Long,   // Monday, January 2, 2026
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontSize {
    Small,      // 1.0x baseline
    Medium,     // 1.22x
    Large,      // 1.45x (Recommended Default)
    ExtraLarge, // 1.70x
    Huge,       // 2.00x
}

impl FontSize {
    pub fn scale(self) -> f32 {
        match self {
            FontSize::Small => 1.0,
            FontSize::Medium => 1.22,
            FontSize::Large => 1.45,
            FontSize::ExtraLarge => 1.70,
            FontSize::Huge => 2.00,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowWidthPreset {
    Compact,   // 350px
    Standard,  // 410px
    Wide,      // 490px (Recommended: Full Hardware Names visible)
    UltraWide, // 580px
}

impl WindowWidthPreset {
    pub fn base_width(self) -> i32 {
        match self {
            WindowWidthPreset::Compact => 350,
            WindowWidthPreset::Standard => 410,
            WindowWidthPreset::Wide => 490,
            WindowWidthPreset::UltraWide => 580,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessSortBy {
    Cpu,
    Memory,
    Disk,
    Network,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DockEdge {
    Left,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    // General / Window Options
    pub poll_interval_ms: u64,
    pub dock_edge: DockEdge,
    pub sidebar_width: i32,
    pub width_preset: WindowWidthPreset,
    pub stay_on_top: bool,
    pub click_through: bool,
    pub show_tray_icon: bool,
    pub run_at_startup: bool,
    pub initially_hidden: bool,
    pub auto_pause_fullscreen: bool,

    // Styling & Theming
    pub theme: AppTheme,
    pub backdrop: BackdropEffect,
    pub font_size: FontSize,
    pub bg_opacity: f32,

    // Header & Clock & Date
    pub show_machine_name: bool,
    pub show_clock: bool,
    pub clock_24hr: bool,
    pub date_format: DateFormat,

    // Telemetry Units & Format
    pub temperature_unit: TemperatureUnit,
    pub use_ghz: bool,
    pub use_bytes: bool,
    pub show_core_loads: bool,
    pub sort_processes_by: ProcessSortBy,

    // Process Category Toggles & Limits
    #[serde(default = "default_true")]
    pub show_top_cpu: bool,
    #[serde(default = "default_true")]
    pub show_top_ram: bool,
    #[serde(default = "default_true")]
    pub show_top_disk: bool,
    #[serde(default = "default_true")]
    pub show_top_network: bool,
    #[serde(default = "default_limit_category")]
    pub process_limit_per_category: usize,

    // Card Visibility Toggles
    pub show_cpu: bool,
    pub show_gpu: bool,
    pub show_ram: bool,
    pub show_storage: bool,
    pub show_network: bool,
    pub show_audio: bool,
    pub show_processes: bool,
    pub show_virtual_memory: bool,
    pub show_battery: bool,
    pub show_system_overview: bool,
    pub show_sensors_card: bool,

    // Hardware & Sensor Discovery Options
    pub show_disabled_hardware: bool,
    pub show_all_sensors: bool,

    // GPU Specific Options
    pub show_all_gpus: bool,
    pub show_gpu_shared_memory: bool,
    pub show_gpu_temperatures: bool,

    // Power Management
    #[serde(default)]
    pub caffeine_enabled: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: 1000,
            dock_edge: DockEdge::Right,
            sidebar_width: 480,
            width_preset: WindowWidthPreset::Wide,
            stay_on_top: true,
            click_through: false,
            show_tray_icon: true,
            run_at_startup: false,
            initially_hidden: false,
            auto_pause_fullscreen: false,

            theme: AppTheme::Auto,
            backdrop: BackdropEffect::Mica,
            font_size: FontSize::Large,
            bg_opacity: 0.92,

            show_machine_name: true,
            show_clock: true,
            clock_24hr: false,
            date_format: DateFormat::Normal,

            temperature_unit: TemperatureUnit::Celsius,
            use_ghz: true,
            use_bytes: true,
            show_core_loads: true,
            sort_processes_by: ProcessSortBy::Cpu,

            show_top_cpu: true,
            show_top_ram: true,
            show_top_disk: true,
            show_top_network: true,
            process_limit_per_category: 4,

            show_cpu: true,
            show_gpu: true,
            show_ram: true,
            show_storage: true,
            show_network: true,
            show_audio: true,
            show_processes: true,
            show_virtual_memory: true,
            show_battery: true,
            show_system_overview: true,
            show_sensors_card: true,

            show_disabled_hardware: true,
            show_all_sensors: true,

            show_all_gpus: true,
            show_gpu_shared_memory: true,
            show_gpu_temperatures: true,

            caffeine_enabled: false,
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        let mut path = dirs_config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("SidebarNative");
        let _ = fs::create_dir_all(&path);
        path.push("config.json");
        path
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str::<Self>(&content) {
                    return config;
                }
            }
        }
        let default_config = Self::default();
        let _ = default_config.save();
        default_config
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}

fn dirs_config_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(PathBuf::from)
}

fn default_true() -> bool {
    true
}

fn default_limit_category() -> usize {
    4
}
