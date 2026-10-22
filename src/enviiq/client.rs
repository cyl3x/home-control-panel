use std::time::Duration;

use serde::de::DeserializeOwned;
use ureq::Agent;
use ureq::config::Config;

use super::Error;
use super::auth::{self, Credentials, Token};
use crate::config;

const API_URL_BASE: &str = "https://api.prod.envi-iq.de/v1";

/// Only reads, the API can change settings of the devices too, which is left to the app.
pub struct Client {
    agent: Agent,
    credentials: Credentials,
    token: Option<Token>,
}

impl Client {
    pub fn new(config: &config::EnviIq) -> Self {
        Self {
            agent: Config::builder()
                .timeout_global(Some(Duration::from_secs(15)))
                .http_status_as_error(false)
                .build()
                .new_agent(),
            credentials: Credentials {
                username: config.username.clone(),
                // Passwords read from a file usually end with a newline
                password: config.password.trim_end_matches(['\r', '\n']).to_string(),
            },
            token: None,
        }
    }

    /// Get a path below the API base, e.g. `/devices`.
    ///
    /// # Errors
    /// Returns an error if the login or the request fails.
    pub fn get<T: DeserializeOwned>(&mut self, path: &str) -> Result<T, Error> {
        match self.try_get(path) {
            // The token may be revoked before it expires
            Err(Error::Status { status: 401, .. }) => {
                log::warn!("EnviIq: token rejected, logging in again");

                self.token = None;
                self.try_get(path)
            }
            result => result,
        }
    }

    fn try_get<T: DeserializeOwned>(&mut self, path: &str) -> Result<T, Error> {
        let url = format!("{API_URL_BASE}{path}");
        let id_token = self.id_token()?;

        let mut response = self
            .agent
            .get(&url)
            .header("Authorization", id_token)
            .header("Accept", "application/json")
            .call()?;

        if !response.status().is_success() {
            return Err(Error::Status {
                status: response.status().as_u16(),
                url,
                body: response.body_mut().read_to_string().unwrap_or_default(),
            });
        }

        Ok(response.body_mut().read_json()?)
    }

    fn id_token(&mut self) -> Result<String, Error> {
        let token = match self.token.take() {
            Some(token) if !token.expires_soon() => token,
            Some(token) => auth::refresh(&self.agent, &token).or_else(|err| {
                log::warn!("EnviIq: token refresh failed, logging in again: {err}");

                auth::login(&self.agent, &self.credentials)
            })?,
            None => {
                log::info!("EnviIq: logging in");

                auth::login(&self.agent, &self.credentials)?
            }
        };

        let id_token = token.id_token.clone();
        self.token = Some(token);

        Ok(id_token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Print the responses of the read endpoints the app uses, to learn their format:
    /// `ENVIIQ_CONFIG=config.toml cargo test dump_responses -- --ignored --nocapture`
    #[test]
    #[ignore = "talks to the real API"]
    fn dump_responses() {
        let path = std::env::var("ENVIIQ_CONFIG").expect("ENVIIQ_CONFIG is set");
        let config = crate::config::init(path.into()).expect("config is valid");
        let mut client = Client::new(config.enviiq.as_ref().expect("[enviiq] is configured"));

        let mut dump = |path: String| {
            let response = client.get::<serde_json::Value>(&path);
            match &response {
                Ok(body) => println!("GET {path}\n{body:#}\n"),
                Err(err) => println!("GET {path}\n{err}\n"),
            }
            response.ok()
        };

        let today = chrono::Local::now().format("%Y-%m-%d");
        dump("/user".to_string());
        dump(format!("/energy-prices/{today}"));
        dump("/energy-taxes".to_string());

        let devices = dump("/devices".to_string()).unwrap_or_default();

        for device in devices.as_array().into_iter().flatten() {
            let Some(device_id) = device["id"].as_str() else {
                continue;
            };

            // Grid fees, the app adds them and the taxes to the exchange price
            if let Some(provider_id) = device["energyProviderId"].as_str() {
                dump(format!("/energy-providers/{provider_id}/fees/{today}"));
            }

            dump(format!("/devices/{device_id}/measurements"));

            for module in device["modules"].as_array().into_iter().flatten() {
                let Some(module_id) = module["id"].as_str() else {
                    continue;
                };

                dump(format!(
                    "/devices/{device_id}/modules/{module_id}/measurements"
                ));
                dump(format!("/devices/{device_id}/modules/{module_id}/shadow"));
            }
        }
    }
}
