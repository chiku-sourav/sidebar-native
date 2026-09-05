use windows::core::w;
use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ,
};

#[derive(Debug, Clone, Default)]
pub struct BiosInfo {
    pub vendor: String,
    pub version: String,
    pub release_date: String,
    pub is_uefi: bool,
    pub secure_boot: String, // "Enabled" | "Disabled" | "Unsupported"
    pub tpm_version: String, // "2.0" | "1.2" | "None"
    pub motherboard_mfg: String,
    pub motherboard_product: String,
    pub system_family: String,
}

pub struct BiosCollector {
    cached: BiosInfo,
}

impl BiosCollector {
    pub fn new() -> Self {
        Self {
            cached: query_bios_info(),
        }
    }
}

impl super::collector::TelemetryCollector for BiosCollector {
    fn name(&self) -> &'static str {
        "BIOS"
    }

    fn update(
        &mut self,
        snapshot: &mut super::TelemetrySnapshot,
        _config: &crate::config::AppConfig,
    ) {
        snapshot.bios = Some(self.cached.clone());
    }
}

fn query_bios_info() -> BiosInfo {
    let mut info = BiosInfo {
        vendor: "American Megatrends".to_string(),
        version: "UEFI".to_string(),
        release_date: "Recent".to_string(),
        is_uefi: true,
        secure_boot: "Unsupported".to_string(),
        tpm_version: "Not Detected".to_string(),
        motherboard_mfg: "Motherboard".to_string(),
        motherboard_product: "System Board".to_string(),
        system_family: "Desktop/Laptop".to_string(),
    };

    unsafe {
        // 1. Query HKLM\HARDWARE\DESCRIPTION\System\BIOS
        let bios_path = w!("HARDWARE\\DESCRIPTION\\System\\BIOS");
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, bios_path, 0, KEY_READ, &mut hkey).is_ok() {
            if let Some(v) = read_reg_sz(hkey, w!("BIOSVendor")) {
                info.vendor = v;
            }
            if let Some(v) = read_reg_sz(hkey, w!("BIOSVersion")) {
                info.version = v;
            }
            if let Some(v) = read_reg_sz(hkey, w!("BIOSReleaseDate")) {
                info.release_date = v;
            }
            if let Some(v) = read_reg_sz(hkey, w!("BaseBoardManufacturer")) {
                info.motherboard_mfg = v;
            }
            if let Some(v) = read_reg_sz(hkey, w!("BaseBoardProduct")) {
                info.motherboard_product = v;
            }
            if let Some(v) = read_reg_sz(hkey, w!("SystemProductName")) {
                info.system_family = v;
            }
            let _ = RegCloseKey(hkey);
        }

        // 2. Query Secure Boot State
        let sec_path = w!("SYSTEM\\CurrentControlSet\\Control\\SecureBoot\\State");
        let mut sec_key = HKEY::default();
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, sec_path, 0, KEY_READ, &mut sec_key).is_ok() {
            info.is_uefi = true;
            let mut val: u32 = 0;
            let mut len = std::mem::size_of::<u32>() as u32;
            if RegQueryValueExW(
                sec_key,
                w!("UEFISecureBootEnabled"),
                None,
                None,
                Some(&mut val as *mut _ as *mut u8),
                Some(&mut len),
            )
            .is_ok()
            {
                info.secure_boot = if val == 1 {
                    "Enabled".to_string()
                } else {
                    "Disabled".to_string()
                };
            }
            let _ = RegCloseKey(sec_key);
        }

        // 3. Query TPM Version from Measured Boot Integrity Services
        let tpm_path = w!("SYSTEM\\CurrentControlSet\\Control\\IntegrityServices");
        let mut tpm_key = HKEY::default();
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, tpm_path, 0, KEY_READ, &mut tpm_key).is_ok() {
            let mut log_format: u32 = 0;
            let mut len = std::mem::size_of::<u32>() as u32;
            if RegQueryValueExW(
                tpm_key,
                w!("TPMActiveLogFormat"),
                None,
                None,
                Some(&mut log_format as *mut _ as *mut u8),
                Some(&mut len),
            )
            .is_ok()
            {
                info.tpm_version = if log_format == 2 {
                    "2.0".to_string()
                } else if log_format == 1 {
                    "1.2".to_string()
                } else {
                    "Enabled".to_string()
                };
            } else {
                // If key exists but no value, TPM is present on platform
                info.tpm_version = "Present".to_string();
            }
            let _ = RegCloseKey(tpm_key);
        } else {
            // Check Endorsement key as secondary confirmation
            let end_path = w!("SYSTEM\\CurrentControlSet\\Services\\TPM\\WMI\\Endorsement");
            let mut end_key = HKEY::default();
            if RegOpenKeyExW(HKEY_LOCAL_MACHINE, end_path, 0, KEY_READ, &mut end_key).is_ok() {
                info.tpm_version = "2.0".to_string();
                let _ = RegCloseKey(end_key);
            }
        }
    }

    info
}

unsafe fn read_reg_sz(hkey: HKEY, val_name: windows::core::PCWSTR) -> Option<String> {
    let mut buf = [0u8; 256];
    let mut len = buf.len() as u32;
    if RegQueryValueExW(
        hkey,
        val_name,
        None,
        None,
        Some(buf.as_mut_ptr()),
        Some(&mut len),
    )
    .is_ok()
        && len > 0
    {
        let u16_slice = std::slice::from_raw_parts(
            buf.as_ptr() as *const u16,
            (len as usize / 2).saturating_sub(1),
        );
        let s = String::from_utf16_lossy(u16_slice).trim().to_string();
        if !s.is_empty() {
            return Some(s);
        }
    }
    None
}
