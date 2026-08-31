use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, SetForegroundWindow, TrackPopupMenuEx,
    MENU_ITEM_FLAGS, MF_CHECKED, MF_POPUP, MF_SEPARATOR, MF_STRING, MF_UNCHECKED,
    TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RETURNCMD,
};

use super::ids::*;
use crate::config::{
    AppConfig, AppTheme, BackdropEffect, DateFormat, TemperatureUnit,
};

pub unsafe fn show_tray_popup_menu(hwnd: HWND, config: &AppConfig) -> u32 {
    let menu = CreatePopupMenu().unwrap_or_default();
    if menu.0.is_null() {
        return 0;
    }

    let check_flag = |checked: bool| -> MENU_ITEM_FLAGS {
        if checked {
            MENU_ITEM_FLAGS(MF_STRING.0 | MF_CHECKED.0)
        } else {
            MENU_ITEM_FLAGS(MF_STRING.0 | MF_UNCHECKED.0)
        }
    };

    // 1. Show/Hide Flyout
    let _ = AppendMenuW(
        menu,
        MF_STRING,
        ID_TRAY_TOGGLE as usize,
        w!("Show / Hide Flyout"),
    );
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());

    // 2. Themes Submenu
    let theme_menu = CreatePopupMenu().unwrap_or_default();
    let _ = AppendMenuW(
        theme_menu,
        check_flag(config.theme == AppTheme::Auto),
        ID_THEME_AUTO as usize,
        w!("Auto (Windows Theme Sync)"),
    );
    let _ = AppendMenuW(
        theme_menu,
        check_flag(config.theme == AppTheme::DarkSlate),
        ID_THEME_DARK as usize,
        w!("Dark Slate"),
    );
    let _ = AppendMenuW(
        theme_menu,
        check_flag(config.theme == AppTheme::LightMode),
        ID_THEME_LIGHT as usize,
        w!("Light Clean"),
    );
    let _ = AppendMenuW(
        theme_menu,
        check_flag(config.theme == AppTheme::OledBlack),
        ID_THEME_OLED as usize,
        w!("OLED Midnight Black"),
    );
    let _ = AppendMenuW(
        theme_menu,
        check_flag(config.theme == AppTheme::Nord),
        ID_THEME_NORD as usize,
        w!("Nord Arctic Slate"),
    );
    let _ = AppendMenuW(
        theme_menu,
        check_flag(config.theme == AppTheme::Cyberpunk),
        ID_THEME_CYBERPUNK as usize,
        w!("Cyberpunk Neon"),
    );
    let _ = AppendMenuW(menu, MF_POPUP, theme_menu.0 as usize, w!("Theme"));

    // 3. Materials / Backdrop Submenu
    let backdrop_menu = CreatePopupMenu().unwrap_or_default();
    let _ = AppendMenuW(
        backdrop_menu,
        check_flag(config.backdrop == BackdropEffect::Mica),
        ID_BACKDROP_MICA as usize,
        w!("Mica (Windows 11)"),
    );
    let _ = AppendMenuW(
        backdrop_menu,
        check_flag(config.backdrop == BackdropEffect::Acrylic),
        ID_BACKDROP_ACRYLIC as usize,
        w!("Acrylic Blur"),
    );
    let _ = AppendMenuW(
        backdrop_menu,
        check_flag(config.backdrop == BackdropEffect::MicaAlt),
        ID_BACKDROP_MICAALT as usize,
        w!("Mica Alt (Tabbed)"),
    );
    let _ = AppendMenuW(
        backdrop_menu,
        check_flag(config.backdrop == BackdropEffect::None),
        ID_BACKDROP_NONE as usize,
        w!("Solid / None"),
    );
    let _ = AppendMenuW(
        menu,
        MF_POPUP,
        backdrop_menu.0 as usize,
        w!("Backdrop Material"),
    );

    // 4. Polling Interval Submenu
    let poll_menu = CreatePopupMenu().unwrap_or_default();
    let _ = AppendMenuW(
        poll_menu,
        check_flag(config.poll_interval_ms == 500),
        ID_POLL_500MS as usize,
        w!("500 ms (Fast)"),
    );
    let _ = AppendMenuW(
        poll_menu,
        check_flag(config.poll_interval_ms == 1000),
        ID_POLL_1000MS as usize,
        w!("1.0 Second (Default)"),
    );
    let _ = AppendMenuW(
        poll_menu,
        check_flag(config.poll_interval_ms == 2000),
        ID_POLL_2000MS as usize,
        w!("2.0 Seconds"),
    );
    let _ = AppendMenuW(
        poll_menu,
        check_flag(config.poll_interval_ms == 3000),
        ID_POLL_3000MS as usize,
        w!("3.0 Seconds"),
    );
    let _ = AppendMenuW(
        poll_menu,
        check_flag(config.poll_interval_ms == 5000),
        ID_POLL_5000MS as usize,
        w!("5.0 Seconds (Low Power)"),
    );
    let _ = AppendMenuW(menu, MF_POPUP, poll_menu.0 as usize, w!("Polling Interval"));

    // 5. Header, Clock & Date Submenu
    let clock_menu = CreatePopupMenu().unwrap_or_default();
    let _ = AppendMenuW(
        clock_menu,
        check_flag(config.show_clock),
        ID_CLOCK_TOGGLE as usize,
        w!("Show Clock"),
    );
    let _ = AppendMenuW(
        clock_menu,
        check_flag(config.clock_24hr),
        ID_CLOCK_24HR as usize,
        w!("24-Hour Time Format"),
    );
    let _ = AppendMenuW(
        clock_menu,
        check_flag(config.show_machine_name),
        ID_MACHINE_NAME as usize,
        w!("Show Computer Name & OS"),
    );
    let _ = AppendMenuW(clock_menu, MF_SEPARATOR, 0, PCWSTR::null());
    let _ = AppendMenuW(
        clock_menu,
        check_flag(config.date_format == DateFormat::Disabled),
        ID_DATE_DISABLED as usize,
        w!("Date: Disabled"),
    );
    let _ = AppendMenuW(
        clock_menu,
        check_flag(config.date_format == DateFormat::Short),
        ID_DATE_SHORT as usize,
        w!("Date: Short (MM/DD/YYYY)"),
    );
    let _ = AppendMenuW(
        clock_menu,
        check_flag(config.date_format == DateFormat::Normal),
        ID_DATE_NORMAL as usize,
        w!("Date: Normal (Mon, Jan 2)"),
    );
    let _ = AppendMenuW(
        clock_menu,
        check_flag(config.date_format == DateFormat::Long),
        ID_DATE_LONG as usize,
        w!("Date: Long (Monday, Jan 2)"),
    );
    let _ = AppendMenuW(
        menu,
        MF_POPUP,
        clock_menu.0 as usize,
        w!("Clock & Date Header"),
    );

    // 6. Units & Formats Submenu
    let unit_menu = CreatePopupMenu().unwrap_or_default();
    let _ = AppendMenuW(
        unit_menu,
        check_flag(config.temperature_unit == TemperatureUnit::Celsius),
        ID_TEMP_CELSIUS as usize,
        w!("Temperature: Celsius (°C)"),
    );
    let _ = AppendMenuW(
        unit_menu,
        check_flag(config.temperature_unit == TemperatureUnit::Fahrenheit),
        ID_TEMP_FAHRENHEIT as usize,
        w!("Temperature: Fahrenheit (°F)"),
    );
    let _ = AppendMenuW(unit_menu, MF_SEPARATOR, 0, PCWSTR::null());
    let _ = AppendMenuW(
        unit_menu,
        check_flag(config.use_ghz),
        ID_UNIT_GHZ as usize,
        w!("CPU Clock: GHz"),
    );
    let _ = AppendMenuW(
        unit_menu,
        check_flag(!config.use_ghz),
        ID_UNIT_MHZ as usize,
        w!("CPU Clock: MHz"),
    );
    let _ = AppendMenuW(unit_menu, MF_SEPARATOR, 0, PCWSTR::null());
    let _ = AppendMenuW(
        unit_menu,
        check_flag(config.use_bytes),
        ID_SPEED_BYTES as usize,
        w!("Network/Disk: Bytes/s (MB/s)"),
    );
    let _ = AppendMenuW(
        unit_menu,
        check_flag(!config.use_bytes),
        ID_SPEED_BITS as usize,
        w!("Network/Disk: Bits/s (Mbps)"),
    );
    let _ = AppendMenuW(unit_menu, MF_SEPARATOR, 0, PCWSTR::null());
    let _ = AppendMenuW(
        unit_menu,
        check_flag(config.show_core_loads),
        ID_TOGGLE_CORE_LOADS as usize,
        w!("Show Per-Core Utilization Grid"),
    );
    let _ = AppendMenuW(unit_menu, MF_SEPARATOR, 0, PCWSTR::null());
    let _ = AppendMenuW(
        unit_menu,
        check_flag(config.show_top_cpu),
        ID_TOGGLE_PROC_CPU as usize,
        w!("Process List: CPU Usage"),
    );
    let _ = AppendMenuW(
        unit_menu,
        check_flag(config.show_top_ram),
        ID_TOGGLE_PROC_RAM as usize,
        w!("Process List: RAM Memory"),
    );
    let _ = AppendMenuW(
        unit_menu,
        check_flag(config.show_top_disk),
        ID_TOGGLE_PROC_DISK as usize,
        w!("Process List: Disk I/O"),
    );
    let _ = AppendMenuW(
        unit_menu,
        check_flag(config.show_top_network),
        ID_TOGGLE_PROC_NETWORK as usize,
        w!("Process List: Network Usage"),
    );
    let _ = AppendMenuW(menu, MF_POPUP, unit_menu.0 as usize, w!("Units & Display"));

    // 7. Card Visibility & Monitors Submenu
    let card_menu = CreatePopupMenu().unwrap_or_default();
    let _ = AppendMenuW(
        card_menu,
        check_flag(config.show_cpu),
        ID_TOGGLE_CPU as usize,
        w!("Processor (CPU)"),
    );
    let _ = AppendMenuW(
        card_menu,
        check_flag(config.show_gpu),
        ID_TOGGLE_GPU as usize,
        w!("Graphics (GPU)"),
    );
    let _ = AppendMenuW(
        card_menu,
        check_flag(config.show_audio),
        ID_TOGGLE_AUDIO as usize,
        w!("Audio Playback Device"),
    );
    let _ = AppendMenuW(
        card_menu,
        check_flag(config.show_ram),
        ID_TOGGLE_RAM as usize,
        w!("System Memory (RAM)"),
    );
    let _ = AppendMenuW(
        card_menu,
        check_flag(config.show_storage),
        ID_TOGGLE_STORAGE as usize,
        w!("Storage & Drives"),
    );
    let _ = AppendMenuW(
        card_menu,
        check_flag(config.show_network),
        ID_TOGGLE_NETWORK as usize,
        w!("Network I/O & Local IP"),
    );
    let _ = AppendMenuW(
        card_menu,
        check_flag(config.show_processes),
        ID_TOGGLE_PROCESSES as usize,
        w!("Top Processes"),
    );
    let _ = AppendMenuW(
        card_menu,
        check_flag(config.show_virtual_memory),
        ID_TOGGLE_VM as usize,
        w!("Virtual Memory"),
    );
    let _ = AppendMenuW(
        card_menu,
        check_flag(config.show_battery),
        ID_TOGGLE_BATTERY as usize,
        w!("Power & Battery"),
    );
    let _ = AppendMenuW(
        card_menu,
        check_flag(config.show_system_overview),
        ID_TOGGLE_SYSTEM as usize,
        w!("System Overview"),
    );
    let _ = AppendMenuW(
        card_menu,
        check_flag(config.show_sensors_card),
        ID_TOGGLE_SENSORS_CARD as usize,
        w!("Hardware & Sensors Explorer"),
    );
    let _ = AppendMenuW(card_menu, MF_SEPARATOR, 0, PCWSTR::null());
    let _ = AppendMenuW(
        card_menu,
        check_flag(config.show_disabled_hardware),
        ID_TOGGLE_DISABLED_HARDWARE as usize,
        w!("Show Disabled / Offline Hardware & Sensors"),
    );
    let _ = AppendMenuW(card_menu, MF_SEPARATOR, 0, PCWSTR::null());
    let _ = AppendMenuW(
        card_menu,
        check_flag(config.show_all_gpus),
        ID_TOGGLE_ALL_GPUS as usize,
        w!("GPU: Multi-GPU Enumeration"),
    );
    let _ = AppendMenuW(
        card_menu,
        check_flag(config.show_gpu_shared_memory),
        ID_TOGGLE_GPU_SHARED as usize,
        w!("GPU: Shared Memory Breakdown"),
    );
    let _ = AppendMenuW(menu, MF_POPUP, card_menu.0 as usize, w!("Monitors & Cards"));

    // 8. Behavior & Startup Options Submenu
    let opt_menu = CreatePopupMenu().unwrap_or_default();
    let _ = AppendMenuW(
        opt_menu,
        check_flag(config.run_at_startup),
        ID_STARTUP_TOGGLE as usize,
        w!("Run at Windows Startup"),
    );
    let _ = AppendMenuW(
        opt_menu,
        check_flag(config.start_minimized),
        ID_START_MINIMIZED_TOGGLE as usize,
        w!("Start Minimized to Tray"),
    );
    let _ = AppendMenuW(
        opt_menu,
        check_flag(config.auto_pause_fullscreen),
        ID_AUTOPAUSE_TOGGLE as usize,
        w!("Auto-Pause on Fullscreen / Games"),
    );
    let _ = AppendMenuW(
        opt_menu,
        check_flag(config.stay_on_top),
        ID_TOPMOST_TOGGLE as usize,
        w!("Always On Top"),
    );
    let _ = AppendMenuW(
        opt_menu,
        check_flag(config.click_through),
        ID_CLICKTHROUGH_TOGGLE as usize,
        w!("Click-Through (Transparent)"),
    );
    let _ = AppendMenuW(
        opt_menu,
        check_flag(config.caffeine_enabled),
        ID_CAFFEINE_TOGGLE as usize,
        w!("Caffeine Mode (Prevent Sleep)"),
    );
    let _ = AppendMenuW(menu, MF_POPUP, opt_menu.0 as usize, w!("Window & Behavior"));

    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
    let _ = AppendMenuW(
        menu,
        MF_STRING,
        ID_TRAY_OPEN_CONFIG as usize,
        w!("Open Config File (JSON)"),
    );
    let _ = AppendMenuW(menu, MF_STRING, ID_TRAY_LOGS as usize, w!("Open Debug Log"));
    let _ = AppendMenuW(
        menu,
        MF_STRING,
        ID_TRAY_ABOUT as usize,
        w!("About Diagnostics"),
    );
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
    let _ = AppendMenuW(
        menu,
        MF_STRING,
        ID_TRAY_EXIT as usize,
        w!("Exit Application"),
    );

    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);

    let _ = SetForegroundWindow(hwnd);
    let cmd = TrackPopupMenuEx(
        menu,
        TPM_RETURNCMD.0 | TPM_BOTTOMALIGN.0 | TPM_LEFTALIGN.0,
        pt.x,
        pt.y,
        hwnd,
        None,
    );

    let _ = DestroyMenu(theme_menu);
    let _ = DestroyMenu(backdrop_menu);
    let _ = DestroyMenu(poll_menu);
    let _ = DestroyMenu(clock_menu);
    let _ = DestroyMenu(unit_menu);
    let _ = DestroyMenu(card_menu);
    let _ = DestroyMenu(opt_menu);
    let _ = DestroyMenu(menu);

    cmd.0 as u32
}

