use windows::core::{s, w};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

/// Dynamic wrapper for NVIDIA Management Library (nvml.dll)
/// Provides direct hardware querying for NVIDIA GPUs (temperature, PCIe link status).
pub struct NvmlHelper {
    h_module: isize,
    fn_device_get_count: unsafe extern "C" fn(*mut u32) -> i32,
    fn_device_get_handle_by_index: unsafe extern "C" fn(u32, *mut isize) -> i32,
    fn_device_get_name: Option<unsafe extern "C" fn(isize, *mut u8, u32) -> i32>,
    fn_device_get_temperature: unsafe extern "C" fn(isize, u32, *mut u32) -> i32,
    fn_device_get_curr_pcie_gen: Option<unsafe extern "C" fn(isize, *mut u32) -> i32>,
    fn_device_get_curr_pcie_width: Option<unsafe extern "C" fn(isize, *mut u32) -> i32>,
    fn_shutdown: Option<unsafe extern "C" fn() -> i32>,
}

unsafe impl Send for NvmlHelper {}
unsafe impl Sync for NvmlHelper {}

impl NvmlHelper {
    pub fn new() -> Option<Self> {
        unsafe {
            let h_mod = LoadLibraryW(w!("nvml.dll")).ok()?;
            if h_mod.is_invalid() {
                return None;
            }

            type FnNvmlInit = unsafe extern "C" fn() -> i32;
            type FnNvmlDeviceGetCount = unsafe extern "C" fn(*mut u32) -> i32;
            type FnNvmlDeviceGetHandleByIndex = unsafe extern "C" fn(u32, *mut isize) -> i32;
            type FnNvmlDeviceGetName = unsafe extern "C" fn(isize, *mut u8, u32) -> i32;
            type FnNvmlDeviceGetTemperature = unsafe extern "C" fn(isize, u32, *mut u32) -> i32;
            type FnNvmlDeviceGetPcie = unsafe extern "C" fn(isize, *mut u32) -> i32;
            type FnNvmlShutdown = unsafe extern "C" fn() -> i32;

            let p_init = GetProcAddress(h_mod, s!("nvmlInit_v2"))
                .or_else(|| GetProcAddress(h_mod, s!("nvmlInit")))?;
            let p_count = GetProcAddress(h_mod, s!("nvmlDeviceGetCount_v2"))
                .or_else(|| GetProcAddress(h_mod, s!("nvmlDeviceGetCount")))?;
            let p_handle = GetProcAddress(h_mod, s!("nvmlDeviceGetHandleByIndex_v2"))
                .or_else(|| GetProcAddress(h_mod, s!("nvmlDeviceGetHandleByIndex")))?;
            let p_temp = GetProcAddress(h_mod, s!("nvmlDeviceGetTemperature"))?;
            let p_name = GetProcAddress(h_mod, s!("nvmlDeviceGetName"));
            let p_pcie_gen = GetProcAddress(h_mod, s!("nvmlDeviceGetCurrPcieLinkGeneration"))
                .or_else(|| GetProcAddress(h_mod, s!("nvmlDeviceGetMaxPcieLinkGeneration")));
            let p_pcie_width = GetProcAddress(h_mod, s!("nvmlDeviceGetCurrPcieLinkWidth"))
                .or_else(|| GetProcAddress(h_mod, s!("nvmlDeviceGetMaxPcieLinkWidth")));
            let p_shutdown = GetProcAddress(h_mod, s!("nvmlShutdown"));

            let fn_init: FnNvmlInit = std::mem::transmute(p_init);
            if fn_init() != 0 {
                let _ = CloseHandle(HANDLE(h_mod.0 as *mut _));
                return None;
            }

            Some(Self {
                h_module: h_mod.0 as isize,
                fn_device_get_count: std::mem::transmute(p_count),
                fn_device_get_handle_by_index: std::mem::transmute(p_handle),
                fn_device_get_name: p_name.map(|p| std::mem::transmute(p)),
                fn_device_get_temperature: std::mem::transmute(p_temp),
                fn_device_get_curr_pcie_gen: p_pcie_gen.map(|p| std::mem::transmute(p)),
                fn_device_get_curr_pcie_width: p_pcie_width.map(|p| std::mem::transmute(p)),
                fn_shutdown: p_shutdown.map(|p| std::mem::transmute(p)),
            })
        }
    }

    pub fn query_gpus(&self) -> Vec<(String, f32, Option<u8>, Option<u8>)> {
        let mut results = Vec::new();
        unsafe {
            let mut count = 0u32;
            if (self.fn_device_get_count)(&mut count) != 0 || count == 0 {
                return results;
            }

            for i in 0..count {
                let mut handle = 0isize;
                if (self.fn_device_get_handle_by_index)(i, &mut handle) == 0 {
                    let mut temp = 0u32;
                    // 0 = NVML_TEMPERATURE_GPU
                    if (self.fn_device_get_temperature)(handle, 0, &mut temp) == 0 {
                        let mut name = format!("NVIDIA GPU #{}", i);
                        if let Some(fn_name) = self.fn_device_get_name {
                            let mut buf = [0u8; 96];
                            if fn_name(handle, buf.as_mut_ptr(), buf.len() as u32) == 0 {
                                if let Ok(s) =
                                    std::ffi::CStr::from_ptr(buf.as_ptr() as *const i8).to_str()
                                {
                                    let clean = s.trim();
                                    if !clean.is_empty() {
                                        name = clean.to_string();
                                    }
                                }
                            }
                        }

                        let pcie_gen = if let Some(fn_gen) = self.fn_device_get_curr_pcie_gen {
                            let mut g = 0u32;
                            if fn_gen(handle, &mut g) == 0 && g > 0 {
                                Some(g as u8)
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        let pcie_width = if let Some(fn_width) = self.fn_device_get_curr_pcie_width
                        {
                            let mut w = 0u32;
                            if fn_width(handle, &mut w) == 0 && w > 0 {
                                Some(w as u8)
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        results.push((name, temp as f32, pcie_gen, pcie_width));
                    }
                }
            }
        }
        results
    }
}

impl Drop for NvmlHelper {
    fn drop(&mut self) {
        unsafe {
            if let Some(fn_shutdown) = self.fn_shutdown {
                fn_shutdown();
            }
            if self.h_module != 0 {
                let _ = CloseHandle(HANDLE(self.h_module as *mut _));
            }
        }
    }
}
