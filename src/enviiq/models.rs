use chrono::{DateTime, Local};
use serde::Deserialize;

/// Less power is rounded to 0 W, the API's own `isProducing` and `isConsuming` are set at 0 W too
const MIN_POWER: f64 = 1.0;

#[derive(Debug, Clone, Deserialize)]
pub struct Device {
    pub id: String,
    #[serde(default)]
    pub modules: Vec<Module>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Module {
    pub id: String,
    /// `INVERTER`, `HEATPUMP` or `WALLBOX`
    #[serde(rename = "type")]
    pub kind: String,
}

/// Energy of today in kWh as summed up by the inverter, the app shows these as totals.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub production: Option<f64>,
    pub consumption: Option<f64>,
    pub grid_consumption: Option<f64>,
    pub grid_feed: Option<f64>,
    /// Share of the consumption covered by solar power, called "Solarbezug" in the app
    pub share_of_total_power_consumption_in_percent: Option<f64>,
}

/// Live values of a part of the system, which fields are set depends on its type.
///
/// Production is what the part supplies to the house and consumption what it takes,
/// so the grid produces while power is bought and a battery consumes while charging.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Measurement {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub is_online: bool,
    pub current_power_production_in_watt: Option<f64>,
    pub current_power_consumption_in_watt: Option<f64>,
    /// In kWh, despite the name
    pub daily_power_production_in_kilo_watt: Option<f64>,
    /// In kWh, despite the name
    pub daily_power_consumption_in_kilo_watt: Option<f64>,
    pub battery_level_in_percent: Option<f64>,
    /// SG Ready state of a heat pump, e.g. `normal` or `blocked`
    pub command: Option<String>,
}

impl Measurement {
    /// Power supplied to the house in watts.
    pub fn production(&self) -> f64 {
        self.current_power_production_in_watt.unwrap_or_default()
    }

    /// Power taken from the house in watts.
    pub fn consumption(&self) -> f64 {
        self.current_power_consumption_in_watt.unwrap_or_default()
    }

    pub fn is_producing(&self) -> bool {
        self.production() >= MIN_POWER
    }

    pub fn is_consuming(&self) -> bool {
        self.consumption() >= MIN_POWER
    }
}

/// Exchange price of a time slot in cents per kWh.
#[derive(Debug, Clone, Deserialize)]
pub struct Price {
    pub from: DateTime<Local>,
    pub to: DateTime<Local>,
    pub price: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_measurements() {
        let measurements: Vec<Measurement> = serde_json::from_str(
            r#"[
                {"batteryLevelInPercent":50.5,"isConsuming":false,"currentPowerConsumptionInWatt":0,"dailyPowerConsumptionInKiloWatt":10,"isProducing":true,"currentPowerProductionInWatt":500,"dailyPowerProductionInKiloWatt":5,"isOnline":true,"type":"BATTERY"},
                {"isConsuming":true,"currentPowerConsumptionInWatt":1000,"dailyPowerConsumptionInKiloWatt":5,"isOnline":true,"command":"normal","type":"HEAT_PUMP"}
            ]"#,
        )
        .unwrap();

        assert_eq!(measurements[0].kind, "BATTERY");
        assert!(measurements[0].is_producing());
        assert!(!measurements[0].is_consuming());
        assert!((measurements[0].production() - 500.0).abs() < f64::EPSILON);
        assert_eq!(measurements[0].battery_level_in_percent, Some(50.5));
        assert_eq!(measurements[1].command.as_deref(), Some("normal"));
        assert!((measurements[1].production()).abs() < f64::EPSILON);
    }

    #[test]
    fn ignores_flags_without_power() {
        let measurement: Measurement = serde_json::from_str(
            r#"{"isConsuming":true,"currentPowerConsumptionInWatt":0,"isProducing":true,"currentPowerProductionInWatt":0.4,"isOnline":true,"type":"ENERGY_GRID"}"#,
        )
        .unwrap();

        assert!(!measurement.is_producing());
        assert!(!measurement.is_consuming());
    }

    #[test]
    fn parses_totals() {
        let totals: Totals = serde_json::from_str(
            r#"{"production":10,"consumption":20.5,"batteryCharge":5,"batteryDischarge":5,"batteryLevel":50,"gridFeed":0,"gridConsumption":10.5,"shareOfTotalPowerConsumptionInPercent":48.78,"isExpired":false}"#,
        )
        .unwrap();

        assert_eq!(totals.consumption, Some(20.5));
        assert_eq!(totals.grid_feed, Some(0.0));
        assert_eq!(
            totals.share_of_total_power_consumption_in_percent,
            Some(48.78)
        );
    }

    #[test]
    fn parses_prices() {
        let prices: Vec<Price> = serde_json::from_str(
            r#"[{"from":"2026-01-01T00:00:00+01:00","to":"2026-01-01T01:00:00+01:00","price":10.5}]"#,
        )
        .unwrap();

        assert_eq!(
            prices[0].from,
            DateTime::parse_from_rfc3339("2025-12-31T23:00:00Z").unwrap()
        );
        assert!((prices[0].price - 10.5).abs() < f64::EPSILON);
    }
}
