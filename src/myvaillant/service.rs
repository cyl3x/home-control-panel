use chrono::{DateTime, Local, Utc};

use super::client::Source;
use super::command::{Action, Command};
use super::models::{self, ControlIdentifier, Home};
use super::{Client, Error};
use crate::config;

pub(super) const QUICK_VETO: &str = "QUICK_VETO";
pub(super) const HOLIDAY: &str = "HOLIDAY";
pub(super) const VENTILATION_BOOST: &str = "VENTILATION_BOOST";
pub(super) const CYLINDER_BOOST: &str = "CYLINDER_BOOST";

/// Fetches the heating systems of an account while keeping the API usage low.
///
/// Homes and control identifiers rarely change and come from the cache,
/// which leaves a single request per system and fetch.
pub struct Service {
    client: Client,
}

#[derive(Debug, Clone)]
pub struct HeatingSystem {
    pub id: String,
    pub control_identifier: ControlIdentifier,
    pub name: String,
    pub outdoor_temperature: Option<f64>,
    pub water_pressure: Option<f64>,
    pub energy_manager_state: Option<String>,
    pub zones: Vec<Zone>,
    pub hot_water: Vec<HotWater>,
    pub circuits: Vec<Circuit>,
}

#[derive(Debug, Clone)]
pub struct Zone {
    pub index: u32,
    pub name: String,
    pub current_temperature: Option<f64>,
    pub current_humidity: Option<f64>,
    pub desired_temperature: Option<f64>,
    pub heating_state: Option<String>,
    pub operation_mode: Option<String>,
    /// Only set if a special function like a quick veto or holiday is active
    pub special_function: Option<String>,
    pub special_function_end: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct HotWater {
    pub index: u32,
    pub current_temperature: Option<f64>,
    pub setpoint: Option<f64>,
    pub min_setpoint: Option<f64>,
    pub max_setpoint: Option<f64>,
    pub operation_mode: Option<String>,
    /// Only set if a special function like a boost is active
    pub special_function: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Circuit {
    pub index: u32,
    pub state: Option<String>,
    pub flow_temperature: Option<f64>,
    pub flow_setpoint: Option<f64>,
}

impl Service {
    pub fn new(config: &config::MyVaillant) -> Self {
        let cache_path =
            std::env::home_dir().map(|path| path.join(".cache/home-control-panel/myvaillant.json"));

        Self {
            client: Client::new(config, cache_path),
        }
    }

    /// The systems of the last fetch and when they were fetched, without contacting the API.
    pub fn cached(&mut self) -> Option<(Vec<HeatingSystem>, DateTime<Local>)> {
        self.systems(Source::Cache)
            .inspect_err(|err| log::info!("MyVaillant: nothing cached to show: {err}"))
            .ok()
    }

    /// Fetch the current state of all supported heating systems.
    ///
    /// # Errors
    /// Returns an error if any request fails or the quota is exceeded.
    pub fn fetch(&mut self) -> Result<Vec<HeatingSystem>, Error> {
        self.systems(Source::Api).map(|(systems, _)| systems)
    }

    /// Send a change to the API.
    ///
    /// # Errors
    /// Returns an error if the request fails or the quota is exceeded.
    pub fn execute(&mut self, command: &Command) -> Result<(), Error> {
        self.client.execute(command)
    }

    fn systems(&mut self, source: Source) -> Result<(Vec<HeatingSystem>, DateTime<Local>), Error> {
        let homes = self.client.homes(source)?;
        let mut systems = Vec::with_capacity(homes.len());
        let mut fetched_at = Utc::now();

        for home in &homes {
            let Some(system_id) = home.system_id.as_deref().filter(|id| !id.is_empty()) else {
                continue;
            };

            let control_identifier = self.client.control_identifier(system_id, source)?;

            if !is_supported(control_identifier) {
                log::info!(
                    "MyVaillant: skipping system {system_id}, {control_identifier:?} is not supported"
                );
                continue;
            }

            let (system, system_fetched_at) =
                self.client.system(system_id, control_identifier, source)?;

            // The oldest response decides how current the data is
            fetched_at = fetched_at.min(system_fetched_at);
            systems.push(HeatingSystem::new(home, control_identifier, system));
        }

        Ok((systems, fetched_at.with_timezone(&Local)))
    }
}

const fn is_supported(control_identifier: ControlIdentifier) -> bool {
    matches!(
        control_identifier,
        ControlIdentifier::Tli | ControlIdentifier::Vrc700
    )
}

impl HeatingSystem {
    /// Merge state, properties and configuration, which are linked by their index.
    fn new(home: &Home, control_identifier: ControlIdentifier, system: models::System) -> Self {
        let models::System {
            mut state,
            mut properties,
            mut configuration,
        } = system;

        let zones = configuration
            .zones
            .into_iter()
            .filter(|config| {
                properties
                    .zones
                    .iter()
                    .find(|properties| properties.index == config.index)
                    .and_then(|properties| properties.is_active)
                    .unwrap_or(true)
            })
            .map(|config| {
                let state =
                    take(&mut state.zones, |state| state.index == config.index).unwrap_or_default();
                Zone::new(config, state)
            })
            .collect();

        let hot_water = state
            .dhw
            .into_iter()
            .map(|state| {
                let config = take(&mut configuration.dhw, |config| config.index == state.index)
                    .unwrap_or_default();
                let properties = take(&mut properties.dhw, |properties| {
                    properties.index == state.index
                })
                .unwrap_or_default();
                HotWater::new(config, properties, state)
            })
            .collect();

        let circuits = state.circuits.into_iter().map(Circuit::from).collect();

        Self {
            id: home.system_id.clone().unwrap_or_default(),
            control_identifier,
            name: home
                .home_name
                .clone()
                .or_else(|| home.product_information.clone())
                .unwrap_or_else(|| "Heizung".to_string()),
            outdoor_temperature: state.system.outdoor_temperature,
            water_pressure: state.system.system_water_pressure,
            energy_manager_state: state.system.energy_manager_state,
            zones,
            hot_water,
            circuits,
        }
    }

    pub fn command(&self, action: Action) -> Command {
        Command {
            system_id: self.id.clone(),
            control_identifier: self.control_identifier,
            action,
        }
    }

    /// Operation modes of zones, which differ between controllers
    pub const fn zone_operation_modes(&self) -> &'static [&'static str] {
        match self.control_identifier {
            ControlIdentifier::Vrc700 => &["AUTO", "DAY", "SET_BACK", "OFF"],
            _ => &["TIME_CONTROLLED", "MANUAL", "OFF"],
        }
    }

    /// Operation modes of hot water, which differ between controllers
    pub const fn hot_water_operation_modes(&self) -> &'static [&'static str] {
        match self.control_identifier {
            ControlIdentifier::Vrc700 => &["AUTO", "DAY", "OFF"],
            _ => &["TIME_CONTROLLED", "MANUAL", "OFF"],
        }
    }

    pub fn is_away(&self) -> bool {
        self.zones
            .iter()
            .any(|zone| zone.special_function.as_deref() == Some(HOLIDAY))
    }

    pub fn is_ventilation_boost(&self) -> bool {
        self.zones
            .iter()
            .any(|zone| zone.special_function.as_deref() == Some(VENTILATION_BOOST))
    }

    pub fn zone(&self, index: u32) -> Option<&Zone> {
        self.zones.iter().find(|zone| zone.index == index)
    }

    pub(super) fn zone_mut(&mut self, index: u32) -> Option<&mut Zone> {
        self.zones.iter_mut().find(|zone| zone.index == index)
    }

    pub fn hot_water(&self, index: u32) -> Option<&HotWater> {
        self.hot_water
            .iter()
            .find(|hot_water| hot_water.index == index)
    }

    pub(super) fn hot_water_mut(&mut self, index: u32) -> Option<&mut HotWater> {
        self.hot_water
            .iter_mut()
            .find(|hot_water| hot_water.index == index)
    }
}

impl Zone {
    pub fn is_quick_veto(&self) -> bool {
        self.special_function.as_deref() == Some(QUICK_VETO)
    }

    fn new(config: models::ZoneConfiguration, state: models::ZoneState) -> Self {
        let special_function = state
            .current_special_function
            .filter(|special_function| special_function != "NONE");

        // Unset dates are sent as sentinels like 1970-01-01, so only future ends are kept
        let special_function_end = match special_function.as_deref() {
            Some(QUICK_VETO) => state.quick_veto_end_date_time,
            Some(HOLIDAY) => config.general.holiday_end_date_time,
            _ => None,
        }
        .filter(|end| *end > Utc::now());

        Self {
            index: config.index,
            name: config
                .general
                .name
                // VRC700 pads names with spaces
                .map(|name| name.trim().to_string())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| format!("Zone {}", config.index + 1)),
            current_temperature: state.current_room_temperature,
            current_humidity: state.current_room_humidity,
            desired_temperature: state.desired_room_temperature_setpoint,
            heating_state: state.heating_state,
            operation_mode: config.heating.operation_mode_heating,
            special_function,
            special_function_end,
        }
    }
}

impl HotWater {
    pub fn is_boost(&self) -> bool {
        self.special_function.as_deref() == Some(CYLINDER_BOOST)
    }

    fn new(
        config: models::DhwConfiguration,
        properties: models::DhwProperties,
        state: models::DhwState,
    ) -> Self {
        Self {
            index: state.index,
            current_temperature: state.current_dhw_temperature,
            setpoint: config.tapping_setpoint,
            min_setpoint: properties.min_setpoint,
            max_setpoint: properties.max_setpoint,
            operation_mode: config.operation_mode_dhw,
            special_function: state.current_special_function.filter(|special_function| {
                !matches!(special_function.as_str(), "NONE" | "REGULAR")
            }),
        }
    }
}

impl From<models::CircuitState> for Circuit {
    fn from(state: models::CircuitState) -> Self {
        Self {
            index: state.index,
            state: state.circuit_state,
            flow_temperature: state.current_circuit_flow_temperature,
            flow_setpoint: state.heating_circuit_flow_setpoint,
        }
    }
}

fn take<T>(items: &mut Vec<T>, predicate: impl Fn(&T) -> bool) -> Option<T> {
    let index = items.iter().position(predicate)?;

    Some(items.swap_remove(index))
}
