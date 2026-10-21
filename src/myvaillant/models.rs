//! Raw responses of the myVAILLANT API.
//!
//! Fields differ between controllers and firmware versions, so everything is optional.
//! VRC700 controllers spell `dhw` as `domesticHotWater`, which is covered by aliases.

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Home {
    pub system_id: Option<String>,
    pub home_name: Option<String>,
    pub product_information: Option<String>,
}

/// Decides which API flavour a system speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ControlIdentifier {
    /// sensoCOMFORT (VRC720) with VR921 or VR940f gateways
    Tli,
    /// multiMATIC VRC700
    Vrc700,
    /// iQconnect, not supported by the system endpoint
    Scf,
    #[serde(other)]
    Unsupported,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlIdentifierResponse {
    pub control_identifier: Option<ControlIdentifier>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct System {
    pub state: State,
    pub properties: Properties,
    pub configuration: Configuration,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct State {
    pub system: SystemState,
    pub zones: Vec<ZoneState>,
    pub circuits: Vec<CircuitState>,
    #[serde(alias = "domesticHotWater")]
    pub dhw: Vec<DhwState>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SystemState {
    pub outdoor_temperature: Option<f64>,
    pub system_water_pressure: Option<f64>,
    /// STANDBY, HEATING, COOLING or DHW
    pub energy_manager_state: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ZoneState {
    pub index: u32,
    pub desired_room_temperature_setpoint: Option<f64>,
    pub current_room_temperature: Option<f64>,
    pub current_room_humidity: Option<f64>,
    /// NONE, QUICK_VETO, HOLIDAY, SYSTEM_OFF, VENTILATION_BOOST or ONE_DAY_BANK_HOLIDAY
    pub current_special_function: Option<String>,
    /// IDLE, HEATING_UP or COOLING_DOWN
    pub heating_state: Option<String>,
    #[serde(deserialize_with = "datetime")]
    pub quick_veto_end_date_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CircuitState {
    pub index: u32,
    /// HEATING, COOLING or STANDBY
    pub circuit_state: Option<String>,
    pub current_circuit_flow_temperature: Option<f64>,
    pub heating_circuit_flow_setpoint: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DhwState {
    pub index: u32,
    /// REGULAR (or NONE), CYLINDER_BOOST, HOLIDAY, SYSTEM_OFF, BANK_HOLIDAY or ONE_DAY_BANK_HOLIDAY
    pub current_special_function: Option<String>,
    #[serde(alias = "currentDomesticHotWaterTemperature")]
    pub current_dhw_temperature: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Properties {
    pub zones: Vec<ZoneProperties>,
    #[serde(alias = "domesticHotWater")]
    pub dhw: Vec<DhwProperties>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ZoneProperties {
    pub index: u32,
    pub is_active: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DhwProperties {
    pub index: u32,
    pub min_setpoint: Option<f64>,
    pub max_setpoint: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Configuration {
    pub zones: Vec<ZoneConfiguration>,
    #[serde(alias = "domesticHotWater")]
    pub dhw: Vec<DhwConfiguration>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ZoneConfiguration {
    pub index: u32,
    pub general: ZoneGeneral,
    pub heating: ZoneHeating,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ZoneGeneral {
    pub name: Option<String>,
    #[serde(alias = "endDateTime", deserialize_with = "datetime")]
    pub holiday_end_date_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ZoneHeating {
    /// TIME_CONTROLLED, MANUAL or OFF (TLI), AUTO, DAY, SET_BACK or OFF (VRC700)
    pub operation_mode_heating: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DhwConfiguration {
    pub index: u32,
    #[serde(alias = "operationModeDomesticHotWater")]
    pub operation_mode_dhw: Option<String>,
    pub tapping_setpoint: Option<f64>,
}

/// Dates are RFC 3339 strings, but some VRC700 systems send unix timestamps.
/// Anything unparsable is treated as unset instead of failing the whole response.
fn datetime<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error> {
    Ok(
        match Option::<serde_json::Value>::deserialize(deserializer)? {
            Some(serde_json::Value::String(text)) => DateTime::parse_from_rfc3339(&text)
                .map(|datetime| datetime.to_utc())
                .ok()
                .or_else(|| {
                    text.parse::<NaiveDateTime>()
                        .ok()
                        .map(|datetime| datetime.and_utc())
                }),
            Some(serde_json::Value::Number(number)) => number
                .as_i64()
                .and_then(|ts| DateTime::from_timestamp(ts, 0)),
            _ => None,
        },
    )
}
