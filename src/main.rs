#![windows_subsystem = "windows"]
#![allow(unused_imports, dead_code, unused_must_use)]
#![allow(
    clippy::too_many_arguments,
    clippy::missing_safety_doc,
    clippy::new_without_default,
    clippy::collapsible_if,
    clippy::field_reassign_with_default
)]

mod config;
mod logger;
mod telemetry;
mod window;

#[cfg(windows)]
#[link(name = "resource", kind = "static")]
extern "C" {}

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32};
use std::sync::Mutex;

use windows::core::w;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::Graphics::Gdi::InvalidateRect;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Power::{
    SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED,
};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::HiDpi::{
    GetDpiForSystem, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, VK_S,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, LoadIconW, RegisterClassExW, SetTimer, ShowWindow,
    TranslateMessage, CS_HREDRAW, CS_VREDRAW, MSG, SW_HIDE, SW_SHOWNOACTIVATE, WNDCLASSEXW,
};

use config::AppConfig;
use logger::Logger;
use telemetry::TelemetryEngine;
use window::{
    apply_window_styling, calculate_window_dimensions, create_flyout_window, get_app_state,
    init_app_state, wnd_proc, AppState, BackdropManager, SystemTray, UIRenderer,
};

fn main() {
    // 1. Initialize logging system
    if let Err(e) = Logger::init() {
        eprintln!("Failed to initialize logger: {:?}", e);
    }

    unsafe {
        // Set Per-Monitor High-DPI Awareness V2
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        log_info!("Checking single instance named mutex...");
        let mutex_name = w!("Local\\SideVitalsNativeMutex");
        let _h_mutex = CreateMutexW(None, true, mutex_name);

        log_info!("Loading application configuration...");
        let config = AppConfig::load();
        let is_dark = BackdropManager::is_system_dark_mode();
        log_info!("Windows system theme detected: is_dark={}", is_dark);

        // Restore caffeine mode if it was enabled before shutdown
        if config.caffeine_enabled {
            log_info!(
                "Caffeine Mode restored from config -> preventing system sleep and display off."
            );
            SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED);
        }

        let init_dpi = GetDpiForSystem();
        log_info!("System DPI detected: {}", init_dpi);
        let font_scale = config.font_size.scale();
        let (win_w, win_h) = calculate_window_dimensions(init_dpi, font_scale);

        log_info!("Initializing hardware telemetry engine with GPU & Thermals monitoring...");
        let telemetry = TelemetryEngine::new();
        telemetry.start(config.clone());

        log_info!("Registering Win32 window class...");
        let h_instance = GetModuleHandleW(None).unwrap_or_default();
        let class_name = w!("SideVitalsFlyoutWindowClass");
        let app_icon =
            LoadIconW(HINSTANCE(h_instance.0), windows::core::PCWSTR(1 as _)).unwrap_or_default();

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: HINSTANCE(h_instance.0),
            hIcon: app_icon,
            hIconSm: app_icon,
            lpszClassName: class_name,
            ..Default::default()
        };

        RegisterClassExW(&wc);

        let hwnd_res =
            create_flyout_window(HINSTANCE(h_instance.0), class_name, &config, win_w, win_h);
        let hwnd = match hwnd_res {
            Ok(h) => {
                log_info!("Successfully created flyout window with HWND: {:?}", h.0);
                h
            }
            Err(e) => {
                log_error!("Failed to create window: {:?}", e);
                return;
            }
        };

        log_info!("Configuring DWM rounded corners and backdrop...");
        apply_window_styling(hwnd, config.backdrop, is_dark);

        log_info!("Creating system tray icon instance...");
        let tray = SystemTray::new(hwnd);
        let renderer = UIRenderer::new(init_dpi, font_scale);
        let start_visible = !config.start_minimized;

        let state = AppState {
            config: Mutex::new(config.clone()),
            telemetry,
            renderer: Mutex::new(renderer),
            tray: Mutex::new(tray),
            is_visible: AtomicBool::new(start_visible),
            is_dark_mode: AtomicBool::new(is_dark),
            scroll_offset_y: AtomicI32::new(0),
            max_scroll_y: AtomicI32::new(0),
            dpi: AtomicU32::new(init_dpi),
            win_width: AtomicI32::new(win_w),
            win_height: AtomicI32::new(win_h),
        };
        let _ = init_app_state(state);

        log_info!("Registering global hotkey Ctrl+Alt+S...");
        let _ = RegisterHotKey(hwnd, 1, MOD_CONTROL | MOD_ALT, VK_S.0 as u32);
        let _ = SetTimer(hwnd, 1, 1000, None);

        if start_visible {
            log_info!("Step 1: Calling ShowWindow(SW_SHOWNOACTIVATE)...");
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            log_info!("Step 2: Calling InvalidateRect...");
            let _ = InvalidateRect(hwnd, None, false);
        } else {
            log_info!("Starting minimized/hidden to system tray.");
            let _ = ShowWindow(hwnd, SW_HIDE);
        }

        log_info!("Step 3: Spawning background tray thread...");
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if let Some(state) = get_app_state() {
                if let Ok(tray) = state.tray.lock() {
                    tray.register();
                }
            }
        });

        log_info!("Step 4: Entering Win32 message loop pump.");
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        log_info!("Win32 message loop exited cleanly. Cleaning up hotkeys and handles.");
        let _ = UnregisterHotKey(hwnd, 1);
        log_info!("SideVitals terminated cleanly.");
    }
}
