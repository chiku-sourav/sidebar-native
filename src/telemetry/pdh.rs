//! Shared PDH (Performance Data Helper) FFI wrapper.
//! Provides a generic counter query helper used by CPU, GPU, Network, and Storage collectors.

#![allow(dead_code)]

use windows::core::{s, w};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

pub type PdhHQuery = isize;
pub type PdhHCounter = isize;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct PdhFmtCounterValue {
    pub c_status: u32,
    pub value: PdhFmtCounterValueUnion,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union PdhFmtCounterValueUnion {
    pub long_value: i32,
    pub double_value: f64,
    pub large_value: i64,
    pub ansi_str_value: *const u8,
    pub wide_str_value: *const u16,
}

pub const PDH_FMT_DOUBLE: u32 = 0x00000200;
pub const PDH_FMT_LARGE: u32 = 0x00000400;
pub const PDH_FMT_NOSCALE: u32 = 0x00001000;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct PdhFmtCounterValueItemW {
    pub sz_name: *mut u16,
    pub fmt_value: PdhFmtCounterValue,
}

pub struct PdhInstanceValue {
    pub name: String,
    pub value: f64,
}

type FnPdhOpenQueryW = unsafe extern "system" fn(*const u16, usize, *mut PdhHQuery) -> u32;
type FnPdhAddEnglishCounterW =
    unsafe extern "system" fn(PdhHQuery, *const u16, usize, *mut PdhHCounter) -> u32;
type FnPdhCollectQueryData = unsafe extern "system" fn(PdhHQuery) -> u32;
type FnPdhGetFormattedCounterValue =
    unsafe extern "system" fn(PdhHCounter, u32, *mut u32, *mut PdhFmtCounterValue) -> u32;
type FnPdhGetFormattedCounterArrayW = unsafe extern "system" fn(
    PdhHCounter,
    u32,
    *mut u32,
    *mut u32,
    *mut PdhFmtCounterValueItemW,
) -> u32;
type FnPdhRemoveCounter = unsafe extern "system" fn(PdhHCounter) -> u32;
type FnPdhCloseQuery = unsafe extern "system" fn(PdhHQuery) -> u32;

/// A general-purpose PDH query wrapper.
pub struct PdhHelper {
    h_module: isize,
    pub h_query: PdhHQuery,
    pub fn_add_counter: FnPdhAddEnglishCounterW,
    pub fn_collect: FnPdhCollectQueryData,
    pub fn_get_value: FnPdhGetFormattedCounterValue,
    pub fn_get_array: Option<FnPdhGetFormattedCounterArrayW>,
    pub fn_remove_counter: FnPdhRemoveCounter,
    pub fn_close: FnPdhCloseQuery,
    pub has_collected_once: bool,
}

unsafe impl Send for PdhHelper {}
unsafe impl Sync for PdhHelper {}

impl PdhHelper {
    pub fn new() -> Option<Self> {
        unsafe {
            let h_module = LoadLibraryW(w!("pdh.dll")).ok()?;
            if h_module.is_invalid() {
                return None;
            }

            let fn_open: FnPdhOpenQueryW =
                std::mem::transmute(GetProcAddress(h_module, s!("PdhOpenQueryW"))?);
            let fn_add_counter: FnPdhAddEnglishCounterW =
                std::mem::transmute(GetProcAddress(h_module, s!("PdhAddEnglishCounterW"))?);
            let fn_collect: FnPdhCollectQueryData =
                std::mem::transmute(GetProcAddress(h_module, s!("PdhCollectQueryData"))?);
            let fn_get_value: FnPdhGetFormattedCounterValue =
                std::mem::transmute(GetProcAddress(h_module, s!("PdhGetFormattedCounterValue"))?);
            let fn_get_array: Option<FnPdhGetFormattedCounterArrayW> =
                GetProcAddress(h_module, s!("PdhGetFormattedCounterArrayW"))
                    .map(|p| std::mem::transmute(p));
            let fn_remove_counter: FnPdhRemoveCounter =
                std::mem::transmute(GetProcAddress(h_module, s!("PdhRemoveCounter"))?);
            let fn_close: FnPdhCloseQuery =
                std::mem::transmute(GetProcAddress(h_module, s!("PdhCloseQuery"))?);

            let mut h_query: PdhHQuery = 0;
            if fn_open(std::ptr::null(), 0, &mut h_query) != 0 || h_query == 0 {
                return None;
            }

            let _ = fn_collect(h_query); // warmup

            Some(Self {
                h_module: h_module.0 as isize,
                h_query,
                fn_add_counter,
                fn_collect,
                fn_get_value,
                fn_get_array,
                fn_remove_counter,
                fn_close,
                has_collected_once: false,
            })
        }
    }

    pub fn add_counter(&mut self, path: &str) -> PdhHCounter {
        let wide: Vec<u16> = format!("{}\0", path).encode_utf16().collect();
        let mut handle: PdhHCounter = 0;
        unsafe {
            let res = (self.fn_add_counter)(self.h_query, wide.as_ptr(), 0, &mut handle);
            if res == 0 {
                handle
            } else {
                0
            }
        }
    }

    pub fn collect(&mut self) -> bool {
        unsafe {
            let status = (self.fn_collect)(self.h_query);
            if status != 0 {
                return false;
            }
            if !self.has_collected_once {
                self.has_collected_once = true;
                return false;
            }
            true
        }
    }

    pub fn read_f64(&self, counter: PdhHCounter) -> f64 {
        if counter == 0 {
            return 0.0;
        }
        unsafe {
            let mut val = PdhFmtCounterValue {
                c_status: 0,
                value: PdhFmtCounterValueUnion { double_value: 0.0 },
            };
            let mut counter_type = 0u32;
            let res = (self.fn_get_value)(
                counter,
                PDH_FMT_DOUBLE | PDH_FMT_NOSCALE,
                &mut counter_type,
                &mut val,
            );
            if res == 0 && val.c_status == 0 {
                val.value.double_value.max(0.0)
            } else {
                0.0
            }
        }
    }

    pub fn read_f32(&self, counter: PdhHCounter) -> f32 {
        self.read_f64(counter) as f32
    }

    pub fn read_u64(&self, counter: PdhHCounter) -> u64 {
        self.read_f64(counter) as u64
    }

    pub fn read_array(&self, counter: PdhHCounter) -> Vec<PdhInstanceValue> {
        let mut results = Vec::new();
        if counter == 0 {
            return results;
        }
        let fn_array = match self.fn_get_array {
            Some(f) => f,
            None => return results,
        };

        unsafe {
            let mut buffer_size = 0u32;
            let mut item_count = 0u32;
            let _ = fn_array(
                counter,
                PDH_FMT_DOUBLE | PDH_FMT_NOSCALE,
                &mut buffer_size,
                &mut item_count,
                std::ptr::null_mut(),
            );

            if buffer_size == 0 {
                return results;
            }

            let mut buffer = vec![0u8; buffer_size as usize];
            let res = fn_array(
                counter,
                PDH_FMT_DOUBLE | PDH_FMT_NOSCALE,
                &mut buffer_size,
                &mut item_count,
                buffer.as_mut_ptr() as *mut PdhFmtCounterValueItemW,
            );

            if res == 0 && item_count > 0 {
                let items_ptr = buffer.as_ptr() as *const PdhFmtCounterValueItemW;
                for i in 0..item_count as usize {
                    let item = &*items_ptr.add(i);
                    let name = if !item.sz_name.is_null() {
                        let mut len = 0;
                        while *item.sz_name.add(len) != 0 {
                            len += 1;
                        }
                        let slice = std::slice::from_raw_parts(item.sz_name, len);
                        String::from_utf16_lossy(slice)
                    } else {
                        String::new()
                    };

                    let val = if item.fmt_value.c_status == 0 {
                        item.fmt_value.value.double_value.max(0.0)
                    } else {
                        0.0
                    };

                    results.push(PdhInstanceValue { name, value: val });
                }
            }
        }
        results
    }
}

impl Drop for PdhHelper {
    fn drop(&mut self) {
        unsafe {
            if self.h_query != 0 {
                (self.fn_close)(self.h_query);
            }
            if self.h_module != 0 {
                let _ = CloseHandle(HANDLE(self.h_module as *mut _));
            }
        }
    }
}
