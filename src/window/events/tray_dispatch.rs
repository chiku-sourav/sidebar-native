use std::sync::atomic::Ordering;
use windows::core::w;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::InvalidateRect;
use windows::Win32::System::Power::{
    SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED,
};
use windows::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, PostQuitMessage, SetWindowPos, ShowWindow, HWND_NOTOPMOST, HWND_TOPMOST,
    MB_ICONINFORMATION, MB_OK, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE,
};

use super::super::backdrop::BackdropManager;
use super::super::flyout::toggle_visibility;
use super::super::startup::StartupManager;
use super::super::state::{AppState, BASE_HEIGHT, BASE_WIDTH};
use crate::config::{
    AppConfig, AppTheme, BackdropEffect, DateFormat, FontSize, TemperatureUnit, WindowWidthPreset,
};
use crate::log_info;
use crate::logger::Logger;
use crate::window::appbar::AppBarManager;
use crate::window::tray::*;

pub unsafe fn handle_tray_menu_action(hwnd: HWND, action: u32, state: &AppState) {
    match action {
        ID_TRAY_TOGGLE => {
            log_info!("Tray Menu -> Show / Hide Flyout selected.");
            toggle_visibility(hwnd);
        }
        ID_THEME_AUTO => {
            log_info!("Theme changed -> Auto (Windows Sync)");
            let mut cfg = state.config.lock().unwrap();
            cfg.theme = AppTheme::Auto;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_THEME_DARK => {
            log_info!("Theme changed -> Dark Slate");
            let mut cfg = state.config.lock().unwrap();
            cfg.theme = AppTheme::DarkSlate;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_THEME_LIGHT => {
            log_info!("Theme changed -> Light Clean");
            let mut cfg = state.config.lock().unwrap();
            cfg.theme = AppTheme::LightMode;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_THEME_OLED => {
            log_info!("Theme changed -> OLED Black");
            let mut cfg = state.config.lock().unwrap();
            cfg.theme = AppTheme::OledBlack;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_THEME_NORD => {
            log_info!("Theme changed -> Nord Arctic");
            let mut cfg = state.config.lock().unwrap();
            cfg.theme = AppTheme::Nord;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_THEME_CYBERPUNK => {
            log_info!("Theme changed -> Cyberpunk Neon");
            let mut cfg = state.config.lock().unwrap();
            cfg.theme = AppTheme::Cyberpunk;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_FONT_SMALL | ID_FONT_MEDIUM | ID_FONT_LARGE | ID_FONT_XLARGE | ID_FONT_HUGE => {
            let new_font_size = match action {
                ID_FONT_SMALL => FontSize::Small,
                ID_FONT_MEDIUM => FontSize::Medium,
                ID_FONT_LARGE => FontSize::Large,
                ID_FONT_XLARGE => FontSize::ExtraLarge,
                ID_FONT_HUGE => FontSize::Huge,
                _ => FontSize::Large,
            };
            log_info!("Font size changed -> {:?}", new_font_size);
            let mut cfg = state.config.lock().unwrap();
            cfg.font_size = new_font_size;
            let _ = cfg.save();
            let font_scale = cfg.font_size.scale();
            drop(cfg);

            let cur_dpi = state.dpi.load(Ordering::Relaxed);
            if let Ok(mut r) = state.renderer.lock() {
                r.update_fonts(cur_dpi, font_scale);
            }
            let dpi_scale = (cur_dpi as f32 / 96.0).max(1.0);
            let new_w =
                (BASE_WIDTH as f32 * dpi_scale * (1.0 + (font_scale - 1.0) * 0.35)).round() as i32;
            let new_h =
                (BASE_HEIGHT as f32 * dpi_scale * (1.0 + (font_scale - 1.0) * 0.25)).round() as i32;
            state.win_width.store(new_w, Ordering::Relaxed);
            state.win_height.store(new_h, Ordering::Relaxed);

            let rect = AppBarManager::get_flyout_rect(new_w, new_h);
            let _ = SetWindowPos(
                hwnd,
                HWND::default(),
                rect.left,
                rect.top,
                new_w,
                new_h,
                SWP_NOACTIVATE,
            );
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_BACKDROP_MICA => {
            log_info!("Backdrop changed -> Mica");
            let mut cfg = state.config.lock().unwrap();
            cfg.backdrop = BackdropEffect::Mica;
            let _ = cfg.save();
            let is_dark = state.is_dark_mode.load(Ordering::Relaxed);
            BackdropManager::apply_backdrop(hwnd, BackdropEffect::Mica, is_dark);
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_BACKDROP_ACRYLIC => {
            log_info!("Backdrop changed -> Acrylic");
            let mut cfg = state.config.lock().unwrap();
            cfg.backdrop = BackdropEffect::Acrylic;
            let _ = cfg.save();
            let is_dark = state.is_dark_mode.load(Ordering::Relaxed);
            BackdropManager::apply_backdrop(hwnd, BackdropEffect::Acrylic, is_dark);
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_BACKDROP_MICAALT => {
            log_info!("Backdrop changed -> Mica Alt");
            let mut cfg = state.config.lock().unwrap();
            cfg.backdrop = BackdropEffect::MicaAlt;
            let _ = cfg.save();
            let is_dark = state.is_dark_mode.load(Ordering::Relaxed);
            BackdropManager::apply_backdrop(hwnd, BackdropEffect::MicaAlt, is_dark);
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_BACKDROP_NONE => {
            log_info!("Backdrop changed -> None / Solid");
            let mut cfg = state.config.lock().unwrap();
            cfg.backdrop = BackdropEffect::None;
            let _ = cfg.save();
            let is_dark = state.is_dark_mode.load(Ordering::Relaxed);
            BackdropManager::apply_backdrop(hwnd, BackdropEffect::None, is_dark);
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_WIDTH_COMPACT => {
            log_info!("Window Width Preset -> Compact (350px)");
            apply_width_preset(hwnd, state, WindowWidthPreset::Compact);
        }
        ID_WIDTH_STANDARD => {
            log_info!("Window Width Preset -> Standard (410px)");
            apply_width_preset(hwnd, state, WindowWidthPreset::Standard);
        }
        ID_WIDTH_WIDE => {
            log_info!("Window Width Preset -> Wide (490px)");
            apply_width_preset(hwnd, state, WindowWidthPreset::Wide);
        }
        ID_WIDTH_ULTRAWIDE => {
            log_info!("Window Width Preset -> UltraWide (580px)");
            apply_width_preset(hwnd, state, WindowWidthPreset::UltraWide);
        }
        ID_TOGGLE_PROC_CPU => {
            let mut cfg = state.config.lock().unwrap();
            cfg.show_top_cpu = !cfg.show_top_cpu;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_TOGGLE_PROC_RAM => {
            let mut cfg = state.config.lock().unwrap();
            cfg.show_top_ram = !cfg.show_top_ram;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_TOGGLE_PROC_DISK => {
            let mut cfg = state.config.lock().unwrap();
            cfg.show_top_disk = !cfg.show_top_disk;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_TOGGLE_PROC_NETWORK => {
            let mut cfg = state.config.lock().unwrap();
            cfg.show_top_network = !cfg.show_top_network;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_POLL_500MS => {
            let mut cfg = state.config.lock().unwrap();
            cfg.poll_interval_ms = 500;
            let _ = cfg.save();
        }
        ID_POLL_1000MS => {
            let mut cfg = state.config.lock().unwrap();
            cfg.poll_interval_ms = 1000;
            let _ = cfg.save();
        }
        ID_POLL_2000MS => {
            let mut cfg = state.config.lock().unwrap();
            cfg.poll_interval_ms = 2000;
            let _ = cfg.save();
        }
        ID_POLL_3000MS => {
            let mut cfg = state.config.lock().unwrap();
            cfg.poll_interval_ms = 3000;
            let _ = cfg.save();
        }
        ID_POLL_5000MS => {
            let mut cfg = state.config.lock().unwrap();
            cfg.poll_interval_ms = 5000;
            let _ = cfg.save();
        }
        ID_CLOCK_TOGGLE => {
            let mut cfg = state.config.lock().unwrap();
            cfg.show_clock = !cfg.show_clock;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_CLOCK_24HR => {
            let mut cfg = state.config.lock().unwrap();
            cfg.clock_24hr = !cfg.clock_24hr;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_MACHINE_NAME => {
            let mut cfg = state.config.lock().unwrap();
            cfg.show_machine_name = !cfg.show_machine_name;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_DATE_DISABLED => {
            let mut cfg = state.config.lock().unwrap();
            cfg.date_format = DateFormat::Disabled;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_DATE_SHORT => {
            let mut cfg = state.config.lock().unwrap();
            cfg.date_format = DateFormat::Short;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_DATE_NORMAL => {
            let mut cfg = state.config.lock().unwrap();
            cfg.date_format = DateFormat::Normal;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_DATE_LONG => {
            let mut cfg = state.config.lock().unwrap();
            cfg.date_format = DateFormat::Long;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_TEMP_CELSIUS => {
            let mut cfg = state.config.lock().unwrap();
            cfg.temperature_unit = TemperatureUnit::Celsius;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_TEMP_FAHRENHEIT => {
            let mut cfg = state.config.lock().unwrap();
            cfg.temperature_unit = TemperatureUnit::Fahrenheit;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_UNIT_GHZ => {
            let mut cfg = state.config.lock().unwrap();
            cfg.use_ghz = true;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_UNIT_MHZ => {
            let mut cfg = state.config.lock().unwrap();
            cfg.use_ghz = false;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_SPEED_BYTES => {
            let mut cfg = state.config.lock().unwrap();
            cfg.use_bytes = true;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_SPEED_BITS => {
            let mut cfg = state.config.lock().unwrap();
            cfg.use_bytes = false;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_TOGGLE_CORE_LOADS => {
            let mut cfg = state.config.lock().unwrap();
            cfg.show_core_loads = !cfg.show_core_loads;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_TOGGLE_CPU => {
            let mut cfg = state.config.lock().unwrap();
            cfg.show_cpu = !cfg.show_cpu;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_TOGGLE_GPU => {
            let mut cfg = state.config.lock().unwrap();
            cfg.show_gpu = !cfg.show_gpu;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_TOGGLE_AUDIO => {
            let mut cfg = state.config.lock().unwrap();
            cfg.show_audio = !cfg.show_audio;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_TOGGLE_RAM => {
            let mut cfg = state.config.lock().unwrap();
            cfg.show_ram = !cfg.show_ram;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_TOGGLE_STORAGE => {
            let mut cfg = state.config.lock().unwrap();
            cfg.show_storage = !cfg.show_storage;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_TOGGLE_NETWORK => {
            let mut cfg = state.config.lock().unwrap();
            cfg.show_network = !cfg.show_network;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_TOGGLE_PROCESSES => {
            let mut cfg = state.config.lock().unwrap();
            cfg.show_processes = !cfg.show_processes;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_TOGGLE_VM => {
            let mut cfg = state.config.lock().unwrap();
            cfg.show_virtual_memory = !cfg.show_virtual_memory;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_TOGGLE_BATTERY => {
            let mut cfg = state.config.lock().unwrap();
            cfg.show_battery = !cfg.show_battery;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_TOGGLE_SYSTEM => {
            let mut cfg = state.config.lock().unwrap();
            cfg.show_system_overview = !cfg.show_system_overview;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_TOGGLE_SENSORS_CARD => {
            let mut cfg = state.config.lock().unwrap();
            cfg.show_sensors_card = !cfg.show_sensors_card;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_TOGGLE_DISABLED_HARDWARE => {
            let mut cfg = state.config.lock().unwrap();
            cfg.show_disabled_hardware = !cfg.show_disabled_hardware;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_TOGGLE_ALL_GPUS => {
            let mut cfg = state.config.lock().unwrap();
            cfg.show_all_gpus = !cfg.show_all_gpus;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_TOGGLE_GPU_SHARED => {
            let mut cfg = state.config.lock().unwrap();
            cfg.show_gpu_shared_memory = !cfg.show_gpu_shared_memory;
            let _ = cfg.save();
            drop(cfg);
            let _ = InvalidateRect(hwnd, None, false);
        }
        ID_STARTUP_TOGGLE => {
            let mut cfg = state.config.lock().unwrap();
            cfg.run_at_startup = !cfg.run_at_startup;
            let _ = StartupManager::set_run_at_startup(cfg.run_at_startup);
            let _ = cfg.save();
        }
        ID_START_MINIMIZED_TOGGLE => {
            let mut cfg = state.config.lock().unwrap();
            cfg.start_minimized = !cfg.start_minimized;
            let _ = cfg.save();
        }
        ID_AUTOPAUSE_TOGGLE => {
            let mut cfg = state.config.lock().unwrap();
            cfg.auto_pause_fullscreen = !cfg.auto_pause_fullscreen;
            let _ = cfg.save();
        }
        ID_TOPMOST_TOGGLE => {
            let mut cfg = state.config.lock().unwrap();
            cfg.stay_on_top = !cfg.stay_on_top;
            let _ = cfg.save();
            let target_hwnd = if cfg.stay_on_top {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            };
            let _ = SetWindowPos(
                hwnd,
                target_hwnd,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
        ID_CAFFEINE_TOGGLE => {
            let mut cfg = state.config.lock().unwrap();
            cfg.caffeine_enabled = !cfg.caffeine_enabled;
            let _ = cfg.save();
            if cfg.caffeine_enabled {
                log_info!("Caffeine Mode enabled -> preventing system sleep and display off.");
                SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED);
            } else {
                log_info!("Caffeine Mode disabled -> restoring normal power management.");
                SetThreadExecutionState(ES_CONTINUOUS);
            }
        }
        ID_TRAY_OPEN_CONFIG => {
            log_info!("Tray Menu -> Open Config File selected.");
            let cfg_path = AppConfig::config_path();
            let _ = std::process::Command::new("notepad.exe")
                .arg(cfg_path)
                .spawn();
        }
        ID_TRAY_LOGS => {
            log_info!("Tray Menu -> Open Debug Log selected.");
            let log_path = Logger::get_log_path();
            let _ = std::process::Command::new("notepad.exe")
                .arg(log_path)
                .spawn();
        }
        ID_TRAY_ABOUT => {
            log_info!("Tray Menu -> About Diagnostics selected.");
            MessageBoxW(
                hwnd,
                w!("SideVitals (Rust)\n\nUltra-low resource Windows 11 Diagnostics Flyout.\nIdle Memory: < 8 MB RAM\nIdle CPU: < 0.1%\n\nFeatures full GPU, CPU, Audio, RAM, Disk, Network, Thermals & Process monitoring with smooth scrolling and high-DPI scaling.\n\nDebug logs: %APPDATA%\\SideVitals\\sidevitals.log"),
                w!("About SideVitals"),
                MB_OK | MB_ICONINFORMATION,
            );
        }
        ID_TRAY_EXIT => {
            log_info!("Tray Menu -> Exit Application selected. Exiting.");
            let _ = ShowWindow(hwnd, SW_HIDE);
            PostQuitMessage(0);
        }
        _ => {}
    }
}

unsafe fn apply_width_preset(hwnd: HWND, state: &AppState, preset: WindowWidthPreset) {
    let mut cfg = state.config.lock().unwrap();
    cfg.width_preset = preset;
    cfg.sidebar_width = preset.base_width();
    let _ = cfg.save();
    let new_w = cfg.sidebar_width;
    state.win_width.store(new_w, Ordering::Relaxed);
    let cur_h = state.win_height.load(Ordering::Relaxed);
    drop(cfg);
    let rect = AppBarManager::get_flyout_rect(new_w, cur_h);
    let _ = SetWindowPos(
        hwnd,
        HWND::default(),
        rect.left,
        rect.top,
        new_w,
        cur_h,
        SWP_NOACTIVATE,
    );
    let _ = InvalidateRect(hwnd, None, false);
}
