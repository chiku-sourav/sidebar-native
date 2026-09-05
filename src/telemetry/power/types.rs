#[derive(Debug, Clone, Default)]
pub struct SingleBatteryInfo {
    pub name: String,
    pub manufacturer: String,
    pub serial_number: String,
    pub chemistry: String,
    pub manufacture_date: Option<String>,
    pub designed_capacity_mwh: u32,
    pub full_charge_capacity_mwh: u32,
    pub current_capacity_mwh: u32,
    pub health_percent: Option<f32>,
    pub wear_percent: Option<f32>,
    pub cycle_count: Option<u32>,
    pub voltage_volts: Option<f32>,
    pub rate_watts: Option<f32>,
    pub temperature_c: Option<f32>,
    pub is_charging: bool,
    pub is_discharging: bool,
    pub is_critical: bool,

    // Advanced Metrics (gated by config.adv_battery)
    pub charge_rate_mw: Option<i32>,
    pub capacity_granularity_1_mwh: u32,
    pub capacity_granularity_2_mwh: u32,
    pub low_capacity_alert_mwh: u32,
    pub warning_capacity_alert_mwh: u32,
}

#[derive(Debug, Clone, Default)]
pub struct BatteryMetrics {
    pub has_battery: bool,
    pub is_charging: bool,
    pub is_discharging: bool,
    pub is_ac_connected: bool,
    pub is_saver_active: bool,
    pub is_critical: bool,
    pub percentage: u8,
    pub life_time_seconds: Option<u32>,
    pub time_remaining_formatted: String,

    // Aggregate capacities & rates
    pub remaining_capacity_mwh: u32,
    pub full_charge_capacity_mwh: u32,
    pub designed_capacity_mwh: u32,
    pub health_percent: Option<f32>,
    pub wear_percent: Option<f32>,
    pub cycle_count: Option<u32>,
    pub rate_watts: Option<f32>,
    pub voltage_volts: Option<f32>,
    pub temperature_c: Option<f32>,

    // Hardware metadata
    pub chemistry: String,
    pub device_name: String,
    pub manufacturer: String,
    pub serial_number: String,
    pub manufacture_date: Option<String>,
    pub power_state_description: String,

    // Advanced Metrics (gated by config.adv_battery)
    pub charge_rate_mw: Option<i32>,
    pub capacity_granularity_1_mwh: u32,
    pub capacity_granularity_2_mwh: u32,
    pub low_capacity_alert_mwh: u32,
    pub warning_capacity_alert_mwh: u32,
    pub power_plan_name: String,

    // Multi-battery list
    pub batteries: Vec<SingleBatteryInfo>,
}
