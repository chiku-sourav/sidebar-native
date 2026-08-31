use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32};
use std::sync::{Mutex, OnceLock};

use crate::config::AppConfig;
use crate::telemetry::TelemetryEngine;
use crate::window::tray::SystemTray;
use crate::window::UIRenderer;

pub const BASE_WIDTH: i32 = 380;
pub const BASE_HEIGHT: i32 = 620;

pub static APP_STATE: OnceLock<AppState> = OnceLock::new();

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub telemetry: TelemetryEngine,
    pub renderer: Mutex<UIRenderer>,
    pub tray: Mutex<SystemTray>,
    pub is_visible: AtomicBool,
    pub is_dark_mode: AtomicBool,
    pub scroll_offset_y: AtomicI32,
    pub max_scroll_y: AtomicI32,
    pub dpi: AtomicU32,
    pub win_width: AtomicI32,
    pub win_height: AtomicI32,
}

pub fn get_app_state() -> Option<&'static AppState> {
    APP_STATE.get()
}

pub fn init_app_state(state: AppState) -> bool {
    APP_STATE.set(state).is_ok()
}
