//! Changes to heating systems, ported from myPyllant's write methods (v0.9.18).
//!
//! Commands are plain data: [`Command::request`] builds the HTTP request and
//! [`Command::apply`] shows the expected result until the API reflects the change.

use chrono::{DateTime, Local, TimeDelta, Utc};
use serde_json::json;

use super::client::{Request, SYSTEM_CONTROL_API_URL_BASE, system_url};
use super::models::ControlIdentifier;
use super::service::{CYLINDER_BOOST, HOLIDAY, HeatingSystem, QUICK_VETO, VENTILATION_BOOST};

/// Default duration of a quick veto in the app
const QUICK_VETO_DURATION: TimeDelta = TimeDelta::hours(3);

/// Away mode needs an end, myPyllant uses a year to mean "until cancelled"
const AWAY_DURATION: TimeDelta = TimeDelta::days(365);

/// VRC700 controllers require a room temperature while away, default of the HA integration
const VRC700_HOLIDAY_SETPOINT: f64 = 10.0;

/// Room temperatures accepted by the controllers
const ZONE_TEMPERATURE_RANGE: (f64, f64) = (5.0, 30.0);

/// Hot water limits for systems that don't report them
const HOT_WATER_TEMPERATURE_RANGE: (f64, f64) = (35.0, 70.0);

#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    pub system_id: String,
    pub control_identifier: ControlIdentifier,
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Set the temperature of a zone in manual mode
    ZoneManualSetpoint {
        zone: u32,
        temperature: f64,
    },
    /// Override the temperature of a zone for a few hours, `running` updates an active one
    ZoneQuickVeto {
        zone: u32,
        temperature: f64,
        running: bool,
    },
    CancelZoneQuickVeto {
        zone: u32,
    },
    ZoneOperationMode {
        zone: u32,
        mode: String,
    },
    HotWaterSetpoint {
        index: u32,
        temperature: f64,
    },
    HotWaterOperationMode {
        index: u32,
        mode: String,
    },
    HotWaterBoost {
        index: u32,
        active: bool,
    },
    /// Pause heating while airing out
    VentilationBoost {
        active: bool,
    },
    AwayMode {
        active: bool,
    },
}

/// A temperature changed step by step, see [`Setpoint::step`] and [`Setpoint::command`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Setpoint {
    Zone { system_id: String, zone: u32 },
    HotWater { system_id: String, index: u32 },
}

impl Command {
    /// Build the request, `now` is the start of an away mode.
    pub fn request(&self, now: DateTime<Local>) -> Request {
        let vrc700 = self.control_identifier == ControlIdentifier::Vrc700;
        let system = system_url(&self.system_id, self.control_identifier);
        // Some VRC700 settings are only accepted by the system control API
        let system_control = format!("{SYSTEM_CONTROL_API_URL_BASE}/systems/{}", self.system_id);
        let quick_veto = |zone: &u32| {
            if vrc700 {
                format!("{system}/zone/{zone}/heating/quick-veto")
            } else {
                format!("{system}/zones/{zone}/quick-veto")
            }
        };

        match &self.action {
            Action::ZoneManualSetpoint { zone, temperature } if vrc700 => Request::patch(
                format!("{system}/zone/{zone}/heating/comfort-room-temperature"),
                json!({ "comfortRoomTemperature": temperature }),
            ),
            Action::ZoneManualSetpoint { zone, temperature } => Request::patch(
                format!("{system}/zones/{zone}/manual-mode-setpoint"),
                json!({ "setpoint": temperature, "type": "HEATING" }),
            ),
            Action::ZoneQuickVeto {
                zone,
                temperature,
                running: true,
            } => Request::patch(
                quick_veto(zone),
                json!({ "desiredRoomTemperatureSetpoint": temperature }),
            ),
            Action::ZoneQuickVeto {
                zone,
                temperature,
                running: false,
            } => Request::post(
                quick_veto(zone),
                json!({
                    "desiredRoomTemperatureSetpoint": temperature,
                    "duration": QUICK_VETO_DURATION.as_seconds_f64() / 3600.0,
                }),
            ),
            Action::CancelZoneQuickVeto { zone } => Request::delete(quick_veto(zone)),
            Action::ZoneOperationMode { zone, mode } if vrc700 => Request::patch(
                format!("{system_control}/zones/{zone}/heating-operation-mode"),
                json!({ "operationMode": mode }),
            ),
            Action::ZoneOperationMode { zone, mode } => Request::patch(
                format!("{system}/zones/{zone}/heating-operation-mode"),
                json!({ "operationMode": mode }),
            ),
            Action::HotWaterSetpoint { index, temperature } if vrc700 => Request::patch(
                format!("{system_control}/domestic-hot-water/{index}/tapping-setpoint"),
                json!({ "setpoint": temperature }),
            ),
            // Other controllers only accept whole degrees
            Action::HotWaterSetpoint { index, temperature } => Request::patch(
                format!("{system}/domestic-hot-water/{index}/temperature"),
                json!({ "setpoint": temperature.round() as i64 }),
            ),
            Action::HotWaterOperationMode { index, mode } => Request::patch(
                format!("{system}/domestic-hot-water/{index}/operation-mode"),
                json!({ "operationMode": mode }),
            ),
            Action::HotWaterBoost { index, active } => toggle(
                *active,
                format!("{system}/domestic-hot-water/{index}/boost"),
            ),
            Action::VentilationBoost { active } => {
                toggle(*active, format!("{system}/ventilation-boost"))
            }
            Action::AwayMode { active } => {
                let url = if vrc700 {
                    format!("{system}/holiday")
                } else {
                    format!("{system}/away-mode")
                };

                if !active {
                    return Request::delete(url);
                }

                let mut body = json!({
                    "startDateTime": api_datetime(now),
                    "endDateTime": api_datetime(now + AWAY_DURATION),
                });

                if vrc700 {
                    body["setpoint"] = json!(VRC700_HOLIDAY_SETPOINT);
                }

                Request::post(url, body)
            }
        }
    }

    /// Show the expected result until the API reflects the change.
    pub fn apply(&self, systems: &mut [HeatingSystem]) {
        let Some(system) = find_mut(systems, &self.system_id) else {
            return;
        };

        match &self.action {
            Action::ZoneManualSetpoint { zone, temperature } => {
                if let Some(zone) = system.zone_mut(*zone) {
                    zone.desired_temperature = Some(*temperature);
                }
            }
            Action::ZoneQuickVeto {
                zone,
                temperature,
                running,
            } => {
                if let Some(zone) = system.zone_mut(*zone) {
                    zone.desired_temperature = Some(*temperature);

                    if !running {
                        zone.special_function = Some(QUICK_VETO.to_string());
                        zone.special_function_end = Some(Utc::now() + QUICK_VETO_DURATION);
                    }
                }
            }
            Action::CancelZoneQuickVeto { zone } => {
                if let Some(zone) = system.zone_mut(*zone) {
                    zone.special_function = None;
                    zone.special_function_end = None;
                }
            }
            Action::ZoneOperationMode { zone, mode } => {
                if let Some(zone) = system.zone_mut(*zone) {
                    zone.operation_mode = Some(mode.clone());
                }
            }
            Action::HotWaterSetpoint { index, temperature } => {
                if let Some(hot_water) = system.hot_water_mut(*index) {
                    hot_water.setpoint = Some(*temperature);
                }
            }
            Action::HotWaterOperationMode { index, mode } => {
                if let Some(hot_water) = system.hot_water_mut(*index) {
                    hot_water.operation_mode = Some(mode.clone());
                }
            }
            Action::HotWaterBoost { index, active } => {
                if let Some(hot_water) = system.hot_water_mut(*index) {
                    hot_water.special_function = active.then(|| CYLINDER_BOOST.to_string());
                }
            }
            Action::VentilationBoost { active } => {
                set_zone_special_function(system, VENTILATION_BOOST, *active, None);
            }
            Action::AwayMode { active } => {
                set_zone_special_function(
                    system,
                    HOLIDAY,
                    *active,
                    Some(Utc::now() + AWAY_DURATION),
                );
            }
        }
    }
}

impl Setpoint {
    /// The value `steps` steps away from the current one, limited to what the system accepts.
    pub fn step(&self, systems: &[HeatingSystem], steps: i32) -> Option<f64> {
        let (value, step, (min, max)) = match self {
            Self::Zone { system_id, zone } => {
                let zone = find(systems, system_id)?.zone(*zone)?;

                (zone.desired_temperature?, 0.5, ZONE_TEMPERATURE_RANGE)
            }
            Self::HotWater { system_id, index } => {
                let hot_water = find(systems, system_id)?.hot_water(*index)?;
                let range = (
                    hot_water
                        .min_setpoint
                        .unwrap_or(HOT_WATER_TEMPERATURE_RANGE.0),
                    hot_water
                        .max_setpoint
                        .unwrap_or(HOT_WATER_TEMPERATURE_RANGE.1),
                );

                (hot_water.setpoint?, 1.0, range)
            }
        };

        // The API may report values between steps, which count as the first step
        let base = if steps > 0 {
            (value / step).floor()
        } else {
            (value / step).ceil()
        };

        Some(((base + f64::from(steps)) * step).clamp(min, max))
    }

    /// Show a value that is not sent yet.
    pub fn set(&self, systems: &mut [HeatingSystem], value: f64) {
        match self {
            Self::Zone { system_id, zone } => {
                if let Some(zone) =
                    find_mut(systems, system_id).and_then(|system| system.zone_mut(*zone))
                {
                    zone.desired_temperature = Some(value);
                }
            }
            Self::HotWater { system_id, index } => {
                if let Some(hot_water) =
                    find_mut(systems, system_id).and_then(|system| system.hot_water_mut(*index))
                {
                    hot_water.setpoint = Some(value);
                }
            }
        }
    }

    /// The command setting the value. Zones in manual mode change their setpoint,
    /// otherwise a quick veto overrides the time program like in the app.
    pub fn command(&self, systems: &[HeatingSystem], value: f64) -> Option<Command> {
        match self {
            Self::Zone { system_id, zone } => {
                let system = find(systems, system_id)?;
                let zone = system.zone(*zone)?;
                let action = if matches!(zone.operation_mode.as_deref(), Some("MANUAL" | "DAY")) {
                    Action::ZoneManualSetpoint {
                        zone: zone.index,
                        temperature: value,
                    }
                } else {
                    Action::ZoneQuickVeto {
                        zone: zone.index,
                        temperature: value,
                        running: zone.is_quick_veto(),
                    }
                };

                Some(system.command(action))
            }
            Self::HotWater { system_id, index } => {
                Some(find(systems, system_id)?.command(Action::HotWaterSetpoint {
                    index: *index,
                    temperature: value,
                }))
            }
        }
    }
}

fn toggle(active: bool, url: String) -> Request {
    if active {
        Request::post(url, json!({}))
    } else {
        Request::delete(url)
    }
}

fn set_zone_special_function(
    system: &mut HeatingSystem,
    special_function: &str,
    active: bool,
    end: Option<DateTime<Utc>>,
) {
    for zone in &mut system.zones {
        if active {
            zone.special_function = Some(special_function.to_string());
            zone.special_function_end = end;
        } else if zone.special_function.as_deref() == Some(special_function) {
            zone.special_function = None;
            zone.special_function_end = None;
        }
    }
}

/// Like myPyllant, the local time is sent with a `Z` suffix, which the API is known to accept.
fn api_datetime(datetime: DateTime<Local>) -> String {
    datetime.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn find<'a>(systems: &'a [HeatingSystem], system_id: &str) -> Option<&'a HeatingSystem> {
    systems.iter().find(|system| system.id == system_id)
}

fn find_mut<'a>(
    systems: &'a mut [HeatingSystem],
    system_id: &str,
) -> Option<&'a mut HeatingSystem> {
    systems.iter_mut().find(|system| system.id == system_id)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::super::client::Method;
    use super::super::service::{HotWater, Zone};
    use super::*;

    // Expected requests follow myPyllant's write methods, nothing is sent

    const TLI: &str = "https://api.vaillant-group.com/service-connected-control/end-user-app-api/v1/systems/sys/tli";
    const VRC700: &str =
        "https://api.vaillant-group.com/service-connected-control/vrc700/v1/systems/sys";
    const SYSTEM_CONTROL: &str =
        "https://api.vaillant-group.com/service-connected-control/system-control/v1/systems/sys";

    fn command(control_identifier: ControlIdentifier, action: Action) -> Command {
        Command {
            system_id: "sys".to_string(),
            control_identifier,
            action,
        }
    }

    /// Method, URL and the exact JSON of the request
    fn request(control_identifier: ControlIdentifier, action: Action) -> (Method, String, String) {
        // Summer on both ends, so the away mode can't cross a DST change
        let now = Local.with_ymd_and_hms(2026, 7, 1, 8, 30, 0).unwrap();
        let request = command(control_identifier, action).request(now);

        (
            request.method,
            request.url,
            request
                .body
                .map(|body| body.to_string())
                .unwrap_or_default(),
        )
    }

    fn expected(method: Method, url: &str, body: &str) -> (Method, String, String) {
        (method, url.to_string(), body.to_string())
    }

    #[test]
    fn tli_requests() {
        let tli = |action| request(ControlIdentifier::Tli, action);

        assert_eq!(
            tli(Action::ZoneManualSetpoint {
                zone: 0,
                temperature: 21.5
            }),
            expected(
                Method::Patch,
                &format!("{TLI}/zones/0/manual-mode-setpoint"),
                r#"{"setpoint":21.5,"type":"HEATING"}"#
            )
        );
        assert_eq!(
            tli(Action::ZoneQuickVeto {
                zone: 0,
                temperature: 22.0,
                running: false
            }),
            expected(
                Method::Post,
                &format!("{TLI}/zones/0/quick-veto"),
                r#"{"desiredRoomTemperatureSetpoint":22.0,"duration":3.0}"#
            )
        );
        assert_eq!(
            tli(Action::ZoneQuickVeto {
                zone: 0,
                temperature: 22.5,
                running: true
            }),
            expected(
                Method::Patch,
                &format!("{TLI}/zones/0/quick-veto"),
                r#"{"desiredRoomTemperatureSetpoint":22.5}"#
            )
        );
        assert_eq!(
            tli(Action::CancelZoneQuickVeto { zone: 0 }),
            expected(Method::Delete, &format!("{TLI}/zones/0/quick-veto"), "")
        );
        assert_eq!(
            tli(Action::ZoneOperationMode {
                zone: 0,
                mode: "TIME_CONTROLLED".to_string()
            }),
            expected(
                Method::Patch,
                &format!("{TLI}/zones/0/heating-operation-mode"),
                r#"{"operationMode":"TIME_CONTROLLED"}"#
            )
        );
        assert_eq!(
            tli(Action::HotWaterSetpoint {
                index: 255,
                temperature: 50.4
            }),
            expected(
                Method::Patch,
                &format!("{TLI}/domestic-hot-water/255/temperature"),
                r#"{"setpoint":50}"#
            )
        );
        assert_eq!(
            tli(Action::HotWaterOperationMode {
                index: 255,
                mode: "MANUAL".to_string()
            }),
            expected(
                Method::Patch,
                &format!("{TLI}/domestic-hot-water/255/operation-mode"),
                r#"{"operationMode":"MANUAL"}"#
            )
        );
        assert_eq!(
            tli(Action::HotWaterBoost {
                index: 255,
                active: true
            }),
            expected(
                Method::Post,
                &format!("{TLI}/domestic-hot-water/255/boost"),
                "{}"
            )
        );
        assert_eq!(
            tli(Action::HotWaterBoost {
                index: 255,
                active: false
            }),
            expected(
                Method::Delete,
                &format!("{TLI}/domestic-hot-water/255/boost"),
                ""
            )
        );
        assert_eq!(
            tli(Action::VentilationBoost { active: true }),
            expected(Method::Post, &format!("{TLI}/ventilation-boost"), "{}")
        );
        assert_eq!(
            tli(Action::VentilationBoost { active: false }),
            expected(Method::Delete, &format!("{TLI}/ventilation-boost"), "")
        );
        assert_eq!(
            tli(Action::AwayMode { active: true }),
            expected(
                Method::Post,
                &format!("{TLI}/away-mode"),
                r#"{"startDateTime":"2026-07-01T08:30:00.000Z","endDateTime":"2027-07-01T08:30:00.000Z"}"#
            )
        );
        assert_eq!(
            tli(Action::AwayMode { active: false }),
            expected(Method::Delete, &format!("{TLI}/away-mode"), "")
        );
    }

    #[test]
    fn vrc700_requests() {
        let vrc700 = |action| request(ControlIdentifier::Vrc700, action);

        assert_eq!(
            vrc700(Action::ZoneManualSetpoint {
                zone: 1,
                temperature: 21.5
            }),
            expected(
                Method::Patch,
                &format!("{VRC700}/zone/1/heating/comfort-room-temperature"),
                r#"{"comfortRoomTemperature":21.5}"#
            )
        );
        assert_eq!(
            vrc700(Action::ZoneQuickVeto {
                zone: 1,
                temperature: 22.0,
                running: false
            }),
            expected(
                Method::Post,
                &format!("{VRC700}/zone/1/heating/quick-veto"),
                r#"{"desiredRoomTemperatureSetpoint":22.0,"duration":3.0}"#
            )
        );
        assert_eq!(
            vrc700(Action::CancelZoneQuickVeto { zone: 1 }),
            expected(
                Method::Delete,
                &format!("{VRC700}/zone/1/heating/quick-veto"),
                ""
            )
        );
        assert_eq!(
            vrc700(Action::ZoneOperationMode {
                zone: 1,
                mode: "DAY".to_string()
            }),
            expected(
                Method::Patch,
                &format!("{SYSTEM_CONTROL}/zones/1/heating-operation-mode"),
                r#"{"operationMode":"DAY"}"#
            )
        );
        assert_eq!(
            vrc700(Action::HotWaterSetpoint {
                index: 255,
                temperature: 47.5
            }),
            expected(
                Method::Patch,
                &format!("{SYSTEM_CONTROL}/domestic-hot-water/255/tapping-setpoint"),
                r#"{"setpoint":47.5}"#
            )
        );
        assert_eq!(
            vrc700(Action::HotWaterBoost {
                index: 255,
                active: true
            }),
            expected(
                Method::Post,
                &format!("{VRC700}/domestic-hot-water/255/boost"),
                "{}"
            )
        );
        assert_eq!(
            vrc700(Action::AwayMode { active: true }),
            expected(
                Method::Post,
                &format!("{VRC700}/holiday"),
                r#"{"startDateTime":"2026-07-01T08:30:00.000Z","endDateTime":"2027-07-01T08:30:00.000Z","setpoint":10.0}"#
            )
        );
        assert_eq!(
            vrc700(Action::AwayMode { active: false }),
            expected(Method::Delete, &format!("{VRC700}/holiday"), "")
        );
    }

    fn systems(operation_mode: &str) -> Vec<HeatingSystem> {
        vec![HeatingSystem {
            id: "sys".to_string(),
            control_identifier: ControlIdentifier::Tli,
            name: "Demo".to_string(),
            outdoor_temperature: None,
            water_pressure: None,
            energy_manager_state: None,
            zones: vec![Zone {
                index: 0,
                name: "Zone".to_string(),
                current_temperature: Some(20.0),
                current_humidity: None,
                desired_temperature: Some(21.3),
                heating_state: None,
                operation_mode: Some(operation_mode.to_string()),
                special_function: None,
                special_function_end: None,
            }],
            hot_water: vec![HotWater {
                index: 255,
                current_temperature: Some(45.0),
                setpoint: Some(50.0),
                min_setpoint: Some(35.0),
                max_setpoint: Some(52.0),
                operation_mode: Some("MANUAL".to_string()),
                special_function: None,
            }],
            circuits: Vec::new(),
        }]
    }

    fn zone() -> Setpoint {
        Setpoint::Zone {
            system_id: "sys".to_string(),
            zone: 0,
        }
    }

    #[test]
    fn steps_setpoints_within_limits() {
        let systems = systems("MANUAL");
        let hot_water = Setpoint::HotWater {
            system_id: "sys".to_string(),
            index: 255,
        };

        // Values between steps count as the first step in either direction
        assert_eq!(zone().step(&systems, 1), Some(21.5));
        assert_eq!(zone().step(&systems, -1), Some(21.0));
        assert_eq!(zone().step(&systems, 3), Some(22.5));
        assert_eq!(zone().step(&systems, 100), Some(30.0));
        assert_eq!(hot_water.step(&systems, 1), Some(51.0));
        assert_eq!(hot_water.step(&systems, 5), Some(52.0));
        assert_eq!(hot_water.step(&systems, -100), Some(35.0));
    }

    #[test]
    fn manual_zones_change_their_setpoint() {
        assert_eq!(
            zone()
                .command(&systems("MANUAL"), 21.5)
                .map(|command| command.action),
            Some(Action::ZoneManualSetpoint {
                zone: 0,
                temperature: 21.5
            })
        );
    }

    #[test]
    fn scheduled_zones_start_and_then_update_a_quick_veto() {
        let mut systems = systems("TIME_CONTROLLED");
        let start = zone().command(&systems, 22.0).unwrap();

        assert_eq!(
            start.action,
            Action::ZoneQuickVeto {
                zone: 0,
                temperature: 22.0,
                running: false
            }
        );

        start.apply(&mut systems);

        assert!(systems[0].zones[0].is_quick_veto());
        assert_eq!(systems[0].zones[0].desired_temperature, Some(22.0));
        assert_eq!(
            zone().command(&systems, 22.5).map(|command| command.action),
            Some(Action::ZoneQuickVeto {
                zone: 0,
                temperature: 22.5,
                running: true
            })
        );
    }

    #[test]
    fn applies_system_wide_functions_to_zones() {
        let mut systems = systems("MANUAL");

        command(ControlIdentifier::Tli, Action::AwayMode { active: true }).apply(&mut systems);
        assert!(systems[0].is_away());

        command(
            ControlIdentifier::Tli,
            Action::VentilationBoost { active: false },
        )
        .apply(&mut systems);
        assert!(
            systems[0].is_away(),
            "ending another function must not end away mode"
        );

        command(ControlIdentifier::Tli, Action::AwayMode { active: false }).apply(&mut systems);
        assert!(!systems[0].is_away());
    }
}
