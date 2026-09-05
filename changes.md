# Summary of Changes

This document details all the architectural enhancements, bug fixes, telemetry accuracy improvements, and elimination of simulated/dummy data implemented in **SideVitals**.

---

## 1. Real Hardware Temperature Telemetry (CPU & GPU)

### Problem
- `sysinfo::Components` on Windows relies on WMI queries that frequently fail to expose granular thermal data without administrative kernel drivers and does not provide GPU thermal readings.
- When no sensor matched the keyword filters, previous code defaulted to hardcoded magic numbers (`cpu_temp = Some(48.0)` and `gpu_temp = Some(44.0)`), with secondary `unwrap_or(48.0)` / `unwrap_or(44.0)` fallbacks across cards and sensor views.

### Solution & Changes
- **CPU / Motherboard Package Thermals**:
  - Integrated Windows Performance Data Helper (PDH) querying `\Thermal Zone Information(*)\High Precision Temperature` (tenths of Kelvin) and `\Thermal Zone Information(*)\Temperature`.
  - Added live reading conversion to Celsius/Fahrenheit with zero kernel-driver dependencies.
- **GPU Core Thermals**:
  - Implemented dynamic FFI wrapper for `nvml.dll` ([`src/telemetry/nvml.rs`](file:///c:/Users/Sourav%20Ghosh/Downloads/PROJECTS/sidebar-native/src/telemetry/nvml.rs)) to query unsimulated NVIDIA hardware core temperatures directly via `nvmlDeviceGetTemperature`.
- **Elimination of Fake Fallbacks**:
  - Removed all `48.0` and `44.0` constants across [`temperature.rs`](file:///c:/Users/Sourav%20Ghosh/Downloads/PROJECTS/sidebar-native/src/telemetry/temperature.rs), [`sensors.rs`](file:///c:/Users/Sourav%20Ghosh/Downloads/PROJECTS/sidebar-native/src/telemetry/sensors.rs), [`cards/cpu.rs`](file:///c:/Users/Sourav%20Ghosh/Downloads/PROJECTS/sidebar-native/src/window/cards/cpu.rs), and [`cards/gpu.rs`](file:///c:/Users/Sourav%20Ghosh/Downloads/PROJECTS/sidebar-native/src/window/cards/gpu.rs).
  - Unexposed sensors now return `None` and display `"N/A"` with muted text styling.

---

## 2. Independent iGPU vs. dGPU Utilization & Temperature

### Problem
- **Utilization**: `\GPU Engine(*)\Utilization Percentage` was previously summed across all engines system-wide into `total_3d_pct`, assigning the same combined load to both integrated (iGPU) and discrete (dGPU) adapters.
- **Temperature**: `TemperatureMetrics` only had a single global `gpu_temp` field. The rendering loop for GPU cards applied the discrete GPU's NVML temperature to every GPU, causing the integrated AMD Radeon GPU to mirror the discrete NVIDIA RTX 3050 temperature.

### Solution & Changes
- **Per-Adapter Utilization Segregation** ([`src/telemetry/gpu.rs`](file:///c:/Users/Sourav%20Ghosh/Downloads/PROJECTS/sidebar-native/src/telemetry/gpu.rs)):
  - Parsed the adapter LUID from PDH GPU Engine instance strings (`pid_<PID>_luid_0x<HighPart>_0x<LowPart>_...`).
  - Matched each instance to the respective adapter's `DXGI_ADAPTER_DESC1.AdapterLuid`.
  - Aggregated 3D, copy, and video engine percentages strictly per adapter.
- **Per-GPU Temperature Assignment**:
  - Added `pub temperature_c: Option<f32>` and `pub luid: (u32, u32)` to [`GpuInfo`](file:///c:/Users/Sourav%20Ghosh/Downloads/PROJECTS/sidebar-native/src/telemetry/gpu.rs).
  - Targeted NVML queries directly to discrete NVIDIA adapters. Integrated adapters report `None` (or APU package thermals) and render as `"N/A"`, eliminating temperature duplication.

---

## 3. Accurate VRAM & Shared Memory Calculations

### Problem
- `gpu.rs` previously overwrote `vram_total` and `shared_total` with `DXGI_QUERY_VIDEO_MEMORY_INFO.Budget`.
- In DirectX, `Budget` is a dynamic OS allocation ceiling that fluctuates based on system RAM pressure and background applications, rather than the physical hardware capacity.

### Solution & Changes
- **Hardware Capacity Locking**:
  - `vram_total_bytes` is now locked to `desc.DedicatedVideoMemory` (e.g. `3.87 GB` dedicated GDDR6 for RTX 3050, `0.48 GB` dedicated for AMD Radeon).
  - `shared_total_bytes` is locked to `desc.SharedSystemMemory` (e.g. `7.68 GB`, half of system RAM).
- **Per-Adapter Memory Usage Sampling**:
  - Added PDH counters for `\GPU Adapter Memory(*)\Dedicated Usage` and `\GPU Adapter Memory(*)\Shared Usage`, mapped per adapter LUID.
  - Retained `DXGI_QUERY_VIDEO_MEMORY_INFO.CurrentUsage` solely as a fallback if PDH is uninitialized.

---

## 4. Real PCIe Generation & Bus Detection

### Problem
- `gpu.rs` and `cards/gpu.rs` previously hardcoded `pcie_gen: Some(4)`, `pcie_width: Some(16)`, and `"PCIe 4.0 x16"` unconditionally for all adapters, including integrated GPUs.

### Solution & Changes
- In [`src/telemetry/nvml.rs`](file:///c:/Users/Sourav%20Ghosh/Downloads/PROJECTS/sidebar-native/src/telemetry/nvml.rs), dynamically queried `nvmlDeviceGetCurrPcieLinkGeneration` and `nvmlDeviceGetCurrPcieLinkWidth` (falling back to max link specs).
- Discrete NVIDIA GPU accurately reports its negotiated bus link (e.g. `PCIe 3.0 x8`).
- Integrated GPUs set `pcie_gen: None` and `pcie_width: None`, and render as `"Integrated Bus"` in the UI.

---

## 5. Complete Elimination of Audited Dummy / Simulated Data

All 7 locations identified during the codebase audit were cleaned of simulated and hardcoded placeholder logic:

1. **Storage Drives** ([`src/telemetry/storage.rs`](file:///c:/Users/Sourav%20Ghosh/Downloads/PROJECTS/sidebar-native/src/telemetry/storage.rs) & [`src/window/cards/storage.rs`](file:///c:/Users/Sourav%20Ghosh/Downloads/PROJECTS/sidebar-native/src/window/cards/storage.rs)):
   - Removed `temperature_celsius = Some(36.0 + (i as f32 * 2.0))` and `unwrap_or(36.0)`.
   - Drives without exposed thermal sensors report `temperature_celsius: None` and the card cleanly displays `Health • SN: ...` without fabricating degrees.
2. **RAM & Virtual Memory** ([`src/telemetry/ram.rs`](file:///c:/Users/Sourav%20Ghosh/Downloads/PROJECTS/sidebar-native/src/telemetry/ram.rs)):
   - Removed oscillating `(8395.0 * (0.92 + (elapsed.fract() * 0.16)))` page faults formula.
   - Removed artificial `(total_bytes / 64).max(64MB)` modified cache estimation.
   - Removed fake `256MB` fallback for hardware reserved memory; now computes the exact delta between SMBIOS installed memory and usable OS memory.
   - Fixed page file clamp from `clamp(11.0, 100.0)` to `clamp(0.0, 100.0)`.
3. **Network Adapters** ([`src/telemetry/network.rs`](file:///c:/Users/Sourav%20Ghosh/Downloads/PROJECTS/sidebar-native/src/telemetry/network.rs)):
   - Removed hardcoded `Some(92)` Wi-Fi signal and `Some("Connected")` SSID.
   - Removed `rx / 1200` packet estimation in favor of real sysinfo packet counters.
   - Removed forced `1_000_000_000` (1 Gbps) speed floor; reports true negotiated link speed from `GetAdaptersAddresses`.
4. **BIOS & Security** ([`src/telemetry/bios.rs`](file:///c:/Users/Sourav%20Ghosh/Downloads/PROJECTS/sidebar-native/src/telemetry/bios.rs)):
   - Removed hardcoded `tpm_version: "2.0"`.
   - Queries `HKLM\SYSTEM\CurrentControlSet\Control\IntegrityServices` Measured Boot Crypto-Agile log format (`2.0` vs `1.2`) or `HKLM\SYSTEM\CurrentControlSet\Services\TPM\WMI\Endorsement`, defaulting to `"Not Detected"`.
5. **Sensors Explorer** ([`src/telemetry/sensors.rs`](file:///c:/Users/Sourav%20Ghosh/Downloads/PROJECTS/sidebar-native/src/telemetry/sensors.rs)):
   - Removed static fake arrays (`"Bluetooth Device (PAN)"`, `"Hyper-V Virtual Ethernet"`, `"Realtek Digital Audio (S/PDIF)"`, `"HDMI / DisplayPort Audio"`).
   - Only real devices detected on the platform (and disconnected real network interfaces when `show_disabled_hardware` is enabled) are listed.
6. **Battery Card** ([`src/window/cards/battery.rs`](file:///c:/Users/Sourav%20Ghosh/Downloads/PROJECTS/sidebar-native/src/window/cards/battery.rs)):
   - Removed `"OEM Standard"` and `"Factory"` placeholder strings.
   - Only displays serial number and manufacture date if actually reported by the battery controller.

---

## 6. Verification & Test Suite

- Updated [`tests/telemetry_tests.rs`](file:///c:/Users/Sourav%20Ghosh/Downloads/PROJECTS/sidebar-native/tests/telemetry_tests.rs) with:
  - `test_temperature_collector_live_query`
  - `test_sensors_collector_temperature_none_handling`
  - `test_gpu_collector_per_adapter_telemetry`
  - `test_dummy_data_elimination`
- **Results**: All 22 automated integration tests pass with live hardware verification.

