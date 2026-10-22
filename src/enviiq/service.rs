use chrono::Local;

use super::models::{Device, Measurement, Price, Totals};
use super::{Client, Error};
use crate::config;

/// Fetches the state of the energy system, like the overview of the app.
///
/// The device rarely changes and prices once a day, which leaves two requests per fetch.
pub struct Service {
    client: Client,
    device: Option<Device>,
    /// Prices of the day they were fetched on
    prices: Vec<Price>,
}

#[derive(Debug, Clone, Default)]
pub struct Overview {
    pub photovoltaic: Option<Measurement>,
    pub battery: Option<Measurement>,
    pub grid: Option<Measurement>,
    /// The house without the heat pump
    pub house: Option<Measurement>,
    pub heat_pump: Option<Measurement>,
    /// Energy of today, missing if there is no inverter or fetching it failed
    pub totals: Option<Totals>,
    /// Prices of today, empty if they could not be fetched
    pub prices: Vec<Price>,
}

impl Service {
    pub fn new(config: &config::EnviIq) -> Self {
        Self {
            client: Client::new(config),
            device: None,
            prices: Vec::new(),
        }
    }

    /// Fetch the current measurements, the totals and the prices of today.
    ///
    /// # Errors
    /// Returns an error if the login or fetching the measurements fails.
    pub fn fetch(&mut self) -> Result<Overview, Error> {
        let device = self.device()?;
        let measurements: Vec<Measurement> = self
            .client
            .get(&format!("/devices/{}/measurements", device.id))?;

        let mut overview = Overview {
            totals: self.totals(&device),
            prices: self.prices(),
            ..Overview::default()
        };

        for measurement in measurements {
            let slot = match measurement.kind.as_str() {
                "PHOTO_VOLTAIC" => &mut overview.photovoltaic,
                "BATTERY" => &mut overview.battery,
                "ENERGY_GRID" => &mut overview.grid,
                "CONSUMPTION" => &mut overview.house,
                "HEAT_PUMP" => &mut overview.heat_pump,
                kind => {
                    log::debug!("EnviIq: ignoring measurement of type {kind}");
                    continue;
                }
            };

            *slot = Some(measurement);
        }

        Ok(overview)
    }

    fn device(&mut self) -> Result<Device, Error> {
        if let Some(device) = &self.device {
            return Ok(device.clone());
        }

        let devices: Vec<Device> = self.client.get("/devices")?;

        if devices.len() > 1 {
            log::info!("EnviIq: found {} devices, showing the first", devices.len());
        }

        let device = devices
            .into_iter()
            .next()
            .ok_or_else(|| Error::Parsing("no device in the account".to_string()))?;

        self.device = Some(device.clone());

        Ok(device)
    }

    /// The totals only add to the overview, failing to fetch them is not fatal.
    fn totals(&mut self, device: &Device) -> Option<Totals> {
        let inverter = device
            .modules
            .iter()
            .find(|module| module.kind == "INVERTER")?;

        self.client
            .get(&format!(
                "/devices/{}/modules/{}/measurements",
                device.id, inverter.id
            ))
            .inspect_err(|err| log::warn!("EnviIq: failed to fetch totals: {err}"))
            .ok()
    }

    /// Prices are only fetched again on the next day, failing to fetch them is not fatal.
    fn prices(&mut self) -> Vec<Price> {
        let today = Local::now().date_naive();

        if self
            .prices
            .first()
            .is_some_and(|price| price.from.date_naive() == today)
        {
            return self.prices.clone();
        }

        match self.client.get(&format!("/energy-prices/{today}")) {
            Ok(prices) => self.prices = prices,
            Err(err) => {
                log::warn!("EnviIq: failed to fetch prices: {err}");

                self.prices.clear();
            }
        }

        self.prices.clone()
    }
}
