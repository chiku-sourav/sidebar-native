use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;
use windows::core::{GUID, PCWSTR};
use windows::Win32::System::Diagnostics::Etw::{
    CloseTrace, ControlTraceW, EnableTraceEx2, OpenTraceW, ProcessTrace, StartTraceW,
    CONTROLTRACE_HANDLE, EVENT_RECORD, EVENT_TRACE_CONTROL_STOP, EVENT_TRACE_LOGFILEW,
    EVENT_TRACE_PROPERTIES, EVENT_TRACE_REAL_TIME_MODE, PROCESS_TRACE_MODE_EVENT_RECORD,
    PROCESS_TRACE_MODE_REAL_TIME,
};

// Microsoft-Windows-Kernel-Network GUID: {7dd42a49-5329-4832-8dfd-43d979153a88}
const KERNEL_NETWORK_GUID: GUID = GUID::from_u128(0x7dd42a49_5329_4832_8dfd_43d979153a88);

static ETW_PID_STATS: Mutex<Option<HashMap<u32, (u64, u64)>>> = Mutex::new(None);

unsafe extern "system" fn etw_event_callback(record: *mut EVENT_RECORD) {
    if record.is_null() {
        return;
    }
    let rec = &*record;
    let pid = rec.EventHeader.ProcessId;
    if pid == 0 {
        return;
    }

    let event_id = rec.EventHeader.EventDescriptor.Id;
    let len = rec.UserDataLength as u64;

    // 10 = TcpIpSend, 11 = TcpIpRecv, 12 = TcpIpSendIPv6, 13 = TcpIpRecvIPv6, 42 = UdpSend, 43 = UdpRecv
    let (rx, tx) = match event_id {
        10 | 12 | 42 => (0, if len > 0 { len } else { 64 }),
        11 | 13 | 43 => (if len > 0 { len } else { 64 }, 0),
        _ => return,
    };

    if let Ok(mut guard) = ETW_PID_STATS.lock() {
        if let Some(map) = guard.as_mut() {
            let entry = map.entry(pid).or_insert((0, 0));
            entry.0 += rx;
            entry.1 += tx;
        }
    }
}

pub struct EtwNetworkCollector {
    is_active: bool,
    session_handle: CONTROLTRACE_HANDLE,
    session_name: Vec<u16>,
    last_sample_time: Instant,
}

impl EtwNetworkCollector {
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn new() -> Self {
        let session_name_str = "SidebarNativeNetSession\0";
        let session_name: Vec<u16> = session_name_str.encode_utf16().collect();
        let buf_len = std::mem::size_of::<EVENT_TRACE_PROPERTIES>() + 512;
        let mut buffer = vec![0u8; buf_len];
        let properties = buffer.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;

        let mut session_handle = CONTROLTRACE_HANDLE::default();
        let mut is_active = false;

        unsafe {
            (*properties).Wnode.BufferSize = buf_len as u32;
            (*properties).Wnode.Flags = 0x00020000; // WNODE_FLAG_TRACED_GUID
            (*properties).LogFileMode = EVENT_TRACE_REAL_TIME_MODE;
            (*properties).LoggerNameOffset = std::mem::size_of::<EVENT_TRACE_PROPERTIES>() as u32;

            // Stop any stale session with this name
            let _ = ControlTraceW(
                CONTROLTRACE_HANDLE::default(),
                PCWSTR::from_raw(session_name.as_ptr()),
                properties,
                EVENT_TRACE_CONTROL_STOP,
            );

            let start_res = StartTraceW(
                &mut session_handle,
                PCWSTR::from_raw(session_name.as_ptr()),
                properties,
            );

            if start_res.is_ok() {
                let enable_res = EnableTraceEx2(
                    session_handle,
                    &KERNEL_NETWORK_GUID,
                    1, // EVENT_CONTROL_CODE_ENABLE_PROVIDER
                    4, // TRACE_LEVEL_INFORMATION
                    0xFFFFFFFFFFFFFFFF,
                    0,
                    0,
                    None,
                );

                if enable_res.is_ok() {
                    if let Ok(mut guard) = ETW_PID_STATS.lock() {
                        *guard = Some(HashMap::new());
                    }

                    let sess_name_clone = session_name.clone();
                    std::thread::spawn(move || {
                        let mut logfile = EVENT_TRACE_LOGFILEW::default();
                        logfile.LoggerName =
                            windows::core::PWSTR(sess_name_clone.as_ptr() as *mut u16);
                        logfile.Anonymous1.ProcessTraceMode =
                            PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_EVENT_RECORD;
                        logfile.Anonymous2.EventRecordCallback = Some(etw_event_callback);

                        let trace_handle = OpenTraceW(&mut logfile);
                        if trace_handle.Value != 0 && trace_handle.Value != !0 {
                            let handles = [trace_handle];
                            let _ = ProcessTrace(&handles, None, None);
                            let _ = CloseTrace(trace_handle);
                        }
                    });

                    is_active = true;
                } else {
                    let _ = ControlTraceW(
                        session_handle,
                        PCWSTR::from_raw(session_name.as_ptr()),
                        properties,
                        EVENT_TRACE_CONTROL_STOP,
                    );
                }
            }
        }

        Self {
            is_active,
            session_handle,
            session_name,
            last_sample_time: Instant::now(),
        }
    }

    pub fn sample_and_drain(&mut self) -> HashMap<u32, (u64, u64)> {
        if !self.is_active {
            return HashMap::new();
        }

        let now = Instant::now();
        let elapsed = now
            .duration_since(self.last_sample_time)
            .as_secs_f64()
            .max(0.1);
        self.last_sample_time = now;

        let raw_deltas = {
            let mut guard = match ETW_PID_STATS.lock() {
                Ok(g) => g,
                Err(_) => return HashMap::new(),
            };
            if let Some(map) = guard.as_mut() {
                std::mem::take(map)
            } else {
                HashMap::new()
            }
        };

        let mut rates = HashMap::with_capacity(raw_deltas.len());
        for (pid, (rx, tx)) in raw_deltas {
            let rx_sec = (rx as f64 / elapsed).round() as u64;
            let tx_sec = (tx as f64 / elapsed).round() as u64;
            rates.insert(pid, (rx_sec, tx_sec));
        }
        rates
    }
}

impl Drop for EtwNetworkCollector {
    fn drop(&mut self) {
        if self.is_active {
            unsafe {
                let buf_len = std::mem::size_of::<EVENT_TRACE_PROPERTIES>() + 512;
                let mut buffer = vec![0u8; buf_len];
                let properties = buffer.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;
                (*properties).Wnode.BufferSize = buf_len as u32;
                (*properties).LoggerNameOffset =
                    std::mem::size_of::<EVENT_TRACE_PROPERTIES>() as u32;
                let _ = ControlTraceW(
                    self.session_handle,
                    PCWSTR::from_raw(self.session_name.as_ptr()),
                    properties,
                    EVENT_TRACE_CONTROL_STOP,
                );
            }
        }
    }
}

