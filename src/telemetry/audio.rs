#![allow(unused_imports, dead_code, unused_must_use)]

use windows::core::{Interface, GUID};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    eConsole, eRender, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ,
};
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, PROPERTYKEY};

#[derive(Debug, Clone, Default)]
pub struct AudioMetrics {
    pub device_name: String,
    pub volume_percent: f32,
    pub is_muted: bool,
}

pub struct AudioCollector {}

impl AudioCollector {
    pub fn new() -> Self {
        Self {}
    }

    pub fn collect(&mut self) -> AudioMetrics {
        unsafe {
            let mut metrics = AudioMetrics {
                device_name: "Default Playback Device".to_string(),
                volume_percent: 0.0,
                is_muted: false,
            };

            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

            if let Ok(enumerator) =
                CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)
            {
                if let Ok(device) = enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
                    if let Ok(endpoint_volume) =
                        device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
                    {
                        if let Ok(vol) = endpoint_volume.GetMasterVolumeLevelScalar() {
                            metrics.volume_percent = (vol * 100.0).clamp(0.0, 100.0);
                        }
                        if let Ok(muted) = endpoint_volume.GetMute() {
                            metrics.is_muted = muted.as_bool();
                        }
                    }

                    if let Ok(prop_store) = device.OpenPropertyStore(STGM_READ) {
                        let pkey = PROPERTYKEY {
                            fmtid: GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0),
                            pid: 14,
                        };
                        if let Ok(prop_var) = prop_store.GetValue(&pkey) {
                            let name = prop_var.to_string();
                            if !name.trim().is_empty() {
                                metrics.device_name = name.trim().to_string();
                            }
                        }
                    }
                }
            }

            metrics
        }
    }
}

impl super::collector::TelemetryCollector for AudioCollector {
    fn name(&self) -> &'static str {
        "Audio"
    }

    fn update(
        &mut self,
        snapshot: &mut super::TelemetrySnapshot,
        _config: &crate::config::AppConfig,
    ) {
        snapshot.audio = self.collect();
    }
}
