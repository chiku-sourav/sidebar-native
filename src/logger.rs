use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use windows::Win32::System::SystemInformation::GetLocalTime;

static LOGGER: Mutex<Option<Logger>> = Mutex::new(None);

pub struct Logger {
    file: File,
    log_path: PathBuf,
}

impl Logger {
    pub fn init() -> std::io::Result<()> {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
        let dir = PathBuf::from(appdata).join("SideVitals");
        fs::create_dir_all(&dir)?;

        let log_path = dir.join("sidevitals.log");

        // Rotate if file is larger than 5 MB
        if let Ok(meta) = fs::metadata(&log_path) {
            if meta.len() > 5 * 1024 * 1024 {
                let backup_path = dir.join("sidevitals.old.log");
                let _ = fs::rename(&log_path, backup_path);
            }
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        let logger = Logger { file, log_path };

        let mut lock = LOGGER.lock().unwrap();
        *lock = Some(logger);

        // Install panic hook
        std::panic::set_hook(Box::new(|panic_info| {
            let msg = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                *s
            } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                &s[..]
            } else {
                "Unknown panic payload"
            };

            let location = if let Some(loc) = panic_info.location() {
                format!("{}:{}:{}", loc.file(), loc.line(), loc.column())
            } else {
                "Unknown location".into()
            };

            Logger::log(
                "ERROR",
                "panic",
                &format!("PANIC OCCURRED! Msg: {} at {}", msg, location),
            );
        }));

        drop(lock);

        Logger::log("INFO", "init", "==========================================");
        Logger::log(
            "INFO",
            "init",
            "SideVitals (Rust) Initialized",
        );
        Logger::log(
            "INFO",
            "init",
            &format!("Log path: {}", dir.join("sidevitals.log").display()),
        );
        Logger::log("INFO", "init", "==========================================");

        Ok(())
    }

    pub fn log(level: &str, target: &str, message: &str) {
        let timestamp = get_timestamp();
        let formatted = format!("[{}] [{}] [{}] {}\n", timestamp, level, target, message);

        let mut lock = match LOGGER.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };

        if let Some(logger) = lock.as_mut() {
            let _ = logger.file.write_all(formatted.as_bytes());
            let _ = logger.file.flush();
        }
    }

    pub fn get_log_path() -> PathBuf {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
        PathBuf::from(appdata)
            .join("SideVitals")
            .join("sidevitals.log")
    }
}

fn get_timestamp() -> String {
    unsafe {
        let st = GetLocalTime();
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
            st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond, st.wMilliseconds
        )
    }
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::logger::Logger::log("INFO", module_path!(), &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::logger::Logger::log("DEBUG", module_path!(), &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::logger::Logger::log("WARN", module_path!(), &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::logger::Logger::log("ERROR", module_path!(), &format!($($arg)*))
    };
}
