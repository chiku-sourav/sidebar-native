use windows::Win32::UI::WindowsAndMessaging::WM_USER;

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
pub const ID_CAFFEINE_TOGGLE: u32 = 2705;
pub const ID_START_MINIMIZED_TOGGLE: u32 = 2706;

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

// Process Category Visibility IDs
pub const ID_TOGGLE_PROC_CPU: u32 = 2921;
pub const ID_TOGGLE_PROC_RAM: u32 = 2922;
pub const ID_TOGGLE_PROC_DISK: u32 = 2923;
pub const ID_TOGGLE_PROC_NETWORK: u32 = 2924;

// Advanced Hardware Detail Toggles
pub const ID_ADV_CPU: u32 = 3001;
pub const ID_ADV_GPU: u32 = 3002;
pub const ID_ADV_RAM: u32 = 3003;
pub const ID_ADV_STORAGE: u32 = 3004;
pub const ID_ADV_NETWORK: u32 = 3005;
pub const ID_ADV_BATTERY: u32 = 3006;
pub const ID_ADV_VM: u32 = 3007;
pub const ID_ADV_SENSORS: u32 = 3008;
pub const ID_ADV_BIOS: u32 = 3009;

// Dedicated Caffeine Submenu IDs
pub const ID_CAFFEINE_ENABLE: u32 = 3101;
pub const ID_CAFFEINE_MODE_DISPLAY: u32 = 3102;
pub const ID_CAFFEINE_MODE_SYSTEM: u32 = 3103;
pub const ID_CAFFEINE_SESSION_ONLY: u32 = 3104;
pub const ID_CAFFEINE_TIMEOUT_INDEFINITE: u32 = 3105;
pub const ID_CAFFEINE_TIMEOUT_30M: u32 = 3106;
pub const ID_CAFFEINE_TIMEOUT_1H: u32 = 3107;
pub const ID_CAFFEINE_TIMEOUT_2H: u32 = 3108;
pub const ID_CAFFEINE_TIMEOUT_4H: u32 = 3109;
