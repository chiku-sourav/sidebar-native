#![allow(unused_imports, dead_code, unused_must_use)]

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateBitmap, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreatePen,
    CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, FillRect, FrameRect, GetDC, ReleaseDC,
    RoundRect, SelectObject, SetBkMode, SetTextColor, DT_CENTER, DT_SINGLELINE, DT_VCENTER,
    FW_BOLD, HDC, HFONT, PS_SOLID, TRANSPARENT,
};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconIndirect, CreatePopupMenu, DestroyIcon, DestroyMenu, GetCursorPos,
    SetForegroundWindow, TrackPopupMenuEx, HICON, ICONINFO, MENU_ITEM_FLAGS, MF_CHECKED, MF_POPUP,
    MF_SEPARATOR, MF_STRING, MF_UNCHECKED, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RETURNCMD, WM_USER,
};

use crate::config::{
    AppConfig, AppTheme, BackdropEffect, DateFormat, FontSize, ProcessSortBy, TemperatureUnit,
    WindowWidthPreset,
};

pub const WM_TRAYICON: u32 = WM_USER + 100;
pub const WM_INIT_TRAY: u32 = WM_USER + 101;

pub const ID_TRAY_TOGGLE: u32 = 1001;
pub const ID_TRAY_ABOUT: u32 = 1002;
pub const ID_TRAY_LOGS: u32 = 1003;
pub const ID_TRAY_EXIT: u32 = 1004;
pub const ID_TRAY_OPEN_CONFIG: u32 = 1005;

// Theme Submenu IDs
pub const ID_THEME_AUTO: u32 = 2001;
pub const ID_THEME_DARK: u32 = 2002;
pub const ID_THEME_LIGHT: u32 = 2003;
pub const ID_THEME_OLED: u32 = 2004;
pub const ID_THEME_NORD: u32 = 2005;
pub const ID_THEME_CYBERPUNK: u32 = 2006;

// Backdrop Submenu IDs
pub const ID_BACKDROP_MICA: u32 = 2101;
pub const ID_BACKDROP_ACRYLIC: u32 = 2102;
pub const ID_BACKDROP_MICAALT: u32 = 2103;
pub const ID_BACKDROP_NONE: u32 = 2104;

// Temperature Unit Submenu IDs
pub const ID_TEMP_CELSIUS: u32 = 2201;
pub const ID_TEMP_FAHRENHEIT: u32 = 2202;

// Monitor Card Toggles
pub const ID_TOGGLE_CPU: u32 = 2301;
pub const ID_TOGGLE_GPU: u32 = 2302;
pub const ID_TOGGLE_RAM: u32 = 2303;
pub const ID_TOGGLE_STORAGE: u32 = 2304;
pub const ID_TOGGLE_NETWORK: u32 = 2305;
pub const ID_TOGGLE_PROCESSES: u32 = 2306;
pub const ID_TOGGLE_VM: u32 = 2307;
pub const ID_TOGGLE_BATTERY: u32 = 2308;
pub const ID_TOGGLE_SYSTEM: u32 = 2309;
pub const ID_TOGGLE_ALL_GPUS: u32 = 2310;
pub const ID_TOGGLE_GPU_SHARED: u32 = 2311;
pub const ID_TOGGLE_AUDIO: u32 = 2312;
pub const ID_TOGGLE_CORE_LOADS: u32 = 2313;
pub const ID_TOGGLE_SENSORS_CARD: u32 = 2314;
pub const ID_TOGGLE_DISABLED_HARDWARE: u32 = 2315;

// Polling Rate IDs
pub const ID_POLL_500MS: u32 = 2401;
pub const ID_POLL_1000MS: u32 = 2402;
pub const ID_POLL_2000MS: u32 = 2403;
pub const ID_POLL_3000MS: u32 = 2404;
pub const ID_POLL_5000MS: u32 = 2405;

// Clock & Date Submenu IDs
pub const ID_CLOCK_TOGGLE: u32 = 2501;
pub const ID_CLOCK_24HR: u32 = 2502;
pub const ID_MACHINE_NAME: u32 = 2503;
pub const ID_DATE_DISABLED: u32 = 2504;
pub const ID_DATE_SHORT: u32 = 2505;
pub const ID_DATE_NORMAL: u32 = 2506;
pub const ID_DATE_LONG: u32 = 2507;

// Units & Speed Format IDs
pub const ID_UNIT_GHZ: u32 = 2601;
pub const ID_UNIT_MHZ: u32 = 2602;
pub const ID_SPEED_BYTES: u32 = 2603;
pub const ID_SPEED_BITS: u32 = 2604;

// Options & Behavior IDs
pub const ID_STARTUP_TOGGLE: u32 = 2701;
pub const ID_AUTOPAUSE_TOGGLE: u32 = 2702;
pub const ID_TOPMOST_TOGGLE: u32 = 2703;
pub const ID_CLICKTHROUGH_TOGGLE: u32 = 2704;
pub const ID_CAFFEINE_TOGGLE: u32 = 2705;

// Font Size Submenu IDs
pub const ID_FONT_SMALL: u32 = 2801;
pub const ID_FONT_MEDIUM: u32 = 2802;
pub const ID_FONT_LARGE: u32 = 2803;
pub const ID_FONT_XLARGE: u32 = 2804;
pub const ID_FONT_HUGE: u32 = 2805;

// Window Width Presets IDs
pub const ID_WIDTH_COMPACT: u32 = 2901;
pub const ID_WIDTH_STANDARD: u32 = 2902;
pub const ID_WIDTH_WIDE: u32 = 2903;
pub const ID_WIDTH_ULTRAWIDE: u32 = 2904;

// Process Sorting IDs
pub const ID_PROC_SORT_CPU: u32 = 2911;
pub const ID_PROC_SORT_RAM: u32 = 2912;
pub const ID_PROC_SORT_DISK: u32 = 2913;
pub const ID_PROC_SORT_NETWORK: u32 = 2914;

// Process Category Visibility IDs
pub const ID_TOGGLE_PROC_CPU: u32 = 2921;
pub const ID_TOGGLE_PROC_RAM: u32 = 2922;
pub const ID_TOGGLE_PROC_DISK: u32 = 2923;
pub const ID_TOGGLE_PROC_NETWORK: u32 = 2924;

pub struct SystemTray {
    hwnd: HWND,
    nid: NOTIFYICONDATAW,
    current_icon: HICON,
    last_ram_pct: Option<u8>,
}

unsafe impl Send for SystemTray {}
unsafe impl Sync for SystemTray {}

impl SystemTray {
    pub fn new(hwnd: HWND) -> Self {
        let initial_icon = create_pill_icon(97);

        let mut nid = NOTIFYICONDATAW {
            cbSize: 508, // Standard Windows V2/V3 compatible size
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: initial_icon,
            ..Default::default()
        };

        let tip = "Sidebar Diagnostics (Click to toggle flyout)\0"
            .encode_utf16()
            .collect::<Vec<u16>>();
        let len = tip.len().min(nid.szTip.len());
        nid.szTip[..len].copy_from_slice(&tip[..len]);

        Self {
            hwnd,
            nid,
            current_icon: initial_icon,
            last_ram_pct: Some(97),
        }
    }

    pub fn register(&self) {
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &self.nid);
            let res = Shell_NotifyIconW(NIM_ADD, &self.nid);
            crate::log_info!(
                "System tray icon registered with result: {:?}",
                res.as_bool()
            );
        }
    }

    pub fn update_pill_badge(&mut self, ram_percentage: u8) {
        if self.last_ram_pct == Some(ram_percentage) {
            return;
        }

        self.last_ram_pct = Some(ram_percentage);

        let new_icon = create_pill_icon(ram_percentage);
        if !new_icon.is_invalid() {
            self.nid.hIcon = new_icon;
            unsafe {
                let _ = Shell_NotifyIconW(NIM_MODIFY, &self.nid);
                if !self.current_icon.is_invalid() && self.current_icon != HICON::default() {
                    let _ = DestroyIcon(self.current_icon);
                }
            }
            self.current_icon = new_icon;
        }
    }

    pub fn show_context_menu(&self, config: &AppConfig) -> u32 {
        unsafe {
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

            // 2. Font Size & UI Scale Submenu
            let font_menu = CreatePopupMenu().unwrap_or_default();
            let _ = AppendMenuW(
                font_menu,
                check_flag(config.font_size == FontSize::Small),
                ID_FONT_SMALL as usize,
                w!("Small (100%)"),
            );
            let _ = AppendMenuW(
                font_menu,
                check_flag(config.font_size == FontSize::Medium),
                ID_FONT_MEDIUM as usize,
                w!("Medium (120%)"),
            );
            let _ = AppendMenuW(
                font_menu,
                check_flag(config.font_size == FontSize::Large),
                ID_FONT_LARGE as usize,
                w!("Large (145% - Default)"),
            );
            let _ = AppendMenuW(
                font_menu,
                check_flag(config.font_size == FontSize::ExtraLarge),
                ID_FONT_XLARGE as usize,
                w!("Extra Large (170%)"),
            );
            let _ = AppendMenuW(
                font_menu,
                check_flag(config.font_size == FontSize::Huge),
                ID_FONT_HUGE as usize,
                w!("Huge (200%)"),
            );
            let _ = AppendMenuW(
                menu,
                MF_POPUP,
                font_menu.0 as usize,
                w!("Font Size & Scale"),
            );

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

            // 4. Window Width & Size Submenu
            let width_menu = CreatePopupMenu().unwrap_or_default();
            let _ = AppendMenuW(
                width_menu,
                check_flag(config.width_preset == WindowWidthPreset::Compact),
                ID_WIDTH_COMPACT as usize,
                w!("Compact (350px)"),
            );
            let _ = AppendMenuW(
                width_menu,
                check_flag(config.width_preset == WindowWidthPreset::Standard),
                ID_WIDTH_STANDARD as usize,
                w!("Standard (410px)"),
            );
            let _ = AppendMenuW(
                width_menu,
                check_flag(config.width_preset == WindowWidthPreset::Wide),
                ID_WIDTH_WIDE as usize,
                w!("Wide (490px - Full Hardware Names)"),
            );
            let _ = AppendMenuW(
                width_menu,
                check_flag(config.width_preset == WindowWidthPreset::UltraWide),
                ID_WIDTH_ULTRAWIDE as usize,
                w!("Ultra Wide (580px)"),
            );
            let _ = AppendMenuW(
                menu,
                MF_POPUP,
                width_menu.0 as usize,
                w!("Window Width & Size"),
            );

            // 5. Polling Interval Submenu
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

            // 6. Header, Clock & Date Submenu
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

            // 7. Units & Formats Submenu
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
            let _ = AppendMenuW(unit_menu, MF_SEPARATOR, 0, PCWSTR::null());
            let sort_menu = CreatePopupMenu().unwrap_or_default();
            let _ = AppendMenuW(
                sort_menu,
                check_flag(config.sort_processes_by == ProcessSortBy::Cpu),
                ID_PROC_SORT_CPU as usize,
                w!("Sort by CPU Usage"),
            );
            let _ = AppendMenuW(
                sort_menu,
                check_flag(config.sort_processes_by == ProcessSortBy::Memory),
                ID_PROC_SORT_RAM as usize,
                w!("Sort by Memory (RAM)"),
            );
            let _ = AppendMenuW(
                sort_menu,
                check_flag(config.sort_processes_by == ProcessSortBy::Disk),
                ID_PROC_SORT_DISK as usize,
                w!("Sort by Disk I/O"),
            );
            let _ = AppendMenuW(
                sort_menu,
                check_flag(config.sort_processes_by == ProcessSortBy::Network),
                ID_PROC_SORT_NETWORK as usize,
                w!("Sort by Network Usage"),
            );
            let _ = AppendMenuW(
                unit_menu,
                MF_POPUP,
                sort_menu.0 as usize,
                w!("Process Sort Order"),
            );
            let _ = AppendMenuW(menu, MF_POPUP, unit_menu.0 as usize, w!("Units & Display"));

            // 8. Card Visibility & Monitors Submenu
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

            // 9. Behavior & Startup Options Submenu
            let opt_menu = CreatePopupMenu().unwrap_or_default();
            let _ = AppendMenuW(
                opt_menu,
                check_flag(config.run_at_startup),
                ID_STARTUP_TOGGLE as usize,
                w!("Run at Windows Startup"),
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

            let _ = SetForegroundWindow(self.hwnd);
            let cmd = TrackPopupMenuEx(
                menu,
                TPM_RETURNCMD.0 | TPM_BOTTOMALIGN.0 | TPM_LEFTALIGN.0,
                pt.x,
                pt.y,
                self.hwnd,
                None,
            );

            let _ = DestroyMenu(theme_menu);
            let _ = DestroyMenu(font_menu);
            let _ = DestroyMenu(backdrop_menu);
            let _ = DestroyMenu(width_menu);
            let _ = DestroyMenu(poll_menu);
            let _ = DestroyMenu(clock_menu);
            let _ = DestroyMenu(unit_menu);
            let _ = DestroyMenu(card_menu);
            let _ = DestroyMenu(opt_menu);
            let _ = DestroyMenu(menu);

            cmd.0 as u32
        }
    }
}

fn create_pill_icon(ram_percentage: u8) -> HICON {
    unsafe {
        let screen_dc = GetDC(HWND::default());
        let mem_dc = CreateCompatibleDC(screen_dc);
        let mem_bmp = CreateCompatibleBitmap(screen_dc, 32, 32);
        let old_bmp = SelectObject(mem_dc, mem_bmp);

        let bg_brush = CreateSolidBrush(COLORREF(0x00000000));
        let rect = RECT {
            left: 0,
            top: 0,
            right: 32,
            bottom: 32,
        };
        FillRect(mem_dc, &rect, bg_brush);
        DeleteObject(bg_brush);

        let pill_brush = CreateSolidBrush(COLORREF(0x0032271D));
        let pill_pen = CreatePen(PS_SOLID, 1, COLORREF(0x00F8BD38));
        let old_brush = SelectObject(mem_dc, pill_brush);
        let old_pen = SelectObject(mem_dc, pill_pen);

        let _ = RoundRect(mem_dc, 1, 4, 31, 28, 12, 12);

        SelectObject(mem_dc, old_brush);
        SelectObject(mem_dc, old_pen);
        DeleteObject(pill_brush);
        DeleteObject(pill_pen);

        SetBkMode(mem_dc, TRANSPARENT);
        SetTextColor(mem_dc, COLORREF(0x00F8FAFC));

        let font_name = "Segoe UI\0".encode_utf16().collect::<Vec<u16>>();
        let hfont = CreateFontW(
            -13,
            0,
            0,
            0,
            FW_BOLD.0 as i32,
            0,
            0,
            0,
            1,
            0,
            0,
            0,
            0,
            PCWSTR::from_raw(font_name.as_ptr()),
        );
        let old_font = SelectObject(mem_dc, hfont);

        let mut text_rect = RECT {
            left: 2,
            top: 5,
            right: 30,
            bottom: 27,
        };
        let mut text = format!("{}%\0", ram_percentage)
            .encode_utf16()
            .collect::<Vec<u16>>();
        let text_len = text.len() - 1;
        DrawTextW(
            mem_dc,
            &mut text[..text_len],
            &mut text_rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );

        SelectObject(mem_dc, old_font);
        DeleteObject(hfont);

        // 1-bit monochrome mask bitmap with valid bit array
        let mask_bytes = [0u8; 128]; // 32x32 1-bit = 128 bytes
        let mask_bmp = CreateBitmap(32, 32, 1, 1, Some(mask_bytes.as_ptr() as *const _));

        let icon_info = ICONINFO {
            fIcon: true.into(),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask_bmp,
            hbmColor: mem_bmp,
        };

        let new_icon = CreateIconIndirect(&icon_info).unwrap_or_default();

        SelectObject(mem_dc, old_bmp);
        DeleteObject(mem_bmp);
        DeleteObject(mask_bmp);
        let _ = DeleteDC(mem_dc);
        let _ = ReleaseDC(HWND::default(), screen_dc);

        new_icon
    }
}

impl Drop for SystemTray {
    fn drop(&mut self) {
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &self.nid);
            if !self.current_icon.is_invalid() {
                let _ = DestroyIcon(self.current_icon);
            }
        }
    }
}
