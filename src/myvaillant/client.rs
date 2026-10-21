use std::path::PathBuf;
use std::time::Duration;

use chrono::{DateTime, Local, TimeDelta, Utc};
use serde::de::DeserializeOwned;
use ureq::config::Config;
use ureq::http::Response;
use ureq::{Agent, Body, RequestBuilder};

use super::Error;
use super::auth::{self, Credentials};
use super::cache::Cache;
use super::command::Command;
use super::models::{ControlIdentifier, ControlIdentifierResponse, Home, System};
use crate::config;

const API_URL_BASE: &str =
    "https://api.vaillant-group.com/service-connected-control/end-user-app-api/v1";
const VRC700_API_URL_BASE: &str =
    "https://api.vaillant-group.com/service-connected-control/vrc700/v1";
pub(super) const SYSTEM_CONTROL_API_URL_BASE: &str =
    "https://api.vaillant-group.com/service-connected-control/system-control/v1";

/// Subscription key of the myVAILLANT app
const SUBSCRIPTION_KEY: &str = "1e0a2f3511fb4c5bbb1c7f9fedd20b1c";

/// Pause used if the API does not tell when the quota is replenished
const QUOTA_PAUSE: TimeDelta = TimeDelta::hours(3);

/// Homes and control identifiers practically never change
const STATIC_MAX_AGE: TimeDelta = TimeDelta::days(1);

/// Where a response may come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Only the cache, regardless of the response's age
    Cache,
    /// The API, unless the cache holds a recent response that rarely changes
    Api,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Patch,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub method: Method,
    pub url: String,
    pub body: Option<serde_json::Value>,
}

pub struct Client {
    agent: Agent,
    credentials: Credentials,
    cache: Cache,
}

impl Request {
    pub(super) const fn get(url: String) -> Self {
        Self {
            method: Method::Get,
            url,
            body: None,
        }
    }

    pub(super) const fn post(url: String, body: serde_json::Value) -> Self {
        Self {
            method: Method::Post,
            url,
            body: Some(body),
        }
    }

    pub(super) const fn patch(url: String, body: serde_json::Value) -> Self {
        Self {
            method: Method::Patch,
            url,
            body: Some(body),
        }
    }

    pub(super) const fn delete(url: String) -> Self {
        Self {
            method: Method::Delete,
            url,
            body: None,
        }
    }
}

impl Client {
    /// Create a client, responses are persisted in the given cache file.
    pub fn new(config: &config::MyVaillant, cache_path: Option<PathBuf>) -> Self {
        let credentials = Credentials {
            username: config.username.clone(),
            // Passwords read from a file usually end with a newline
            password: config.password.trim_end_matches(['\r', '\n']).to_string(),
            realm: auth::realm(&config.brand, config.country.as_deref()),
        };

        let account = format!("{}@{}", credentials.username, credentials.realm);

        Self {
            agent: Config::builder()
                .timeout_global(Some(Duration::from_secs(15)))
                // The login needs to read the redirects to the app's URL scheme
                .max_redirects(0)
                .http_status_as_error(false)
                .build()
                .new_agent(),
            credentials,
            cache: Cache::load(cache_path, &account),
        }
    }

    /// Get all homes of the account, each home has one system.
    ///
    /// # Errors
    /// Returns an error if the login or the request fails.
    pub fn homes(&mut self, source: Source) -> Result<Vec<Home>, Error> {
        let (homes, _) = self.get(&format!("{API_URL_BASE}/homes"), source, STATIC_MAX_AGE)?;

        Ok(homes)
    }

    /// Get the controller type of the system, which decides the endpoints to use.
    ///
    /// # Errors
    /// Returns an error if the login or the request fails.
    pub fn control_identifier(
        &mut self,
        system_id: &str,
        source: Source,
    ) -> Result<ControlIdentifier, Error> {
        let (response, _): (ControlIdentifierResponse, _) = self.get(
            &format!("{API_URL_BASE}/systems/{system_id}/meta-info/control-identifier"),
            source,
            STATIC_MAX_AGE,
        )?;

        Ok(response
            .control_identifier
            .unwrap_or(ControlIdentifier::Tli))
    }

    /// Get state, properties and configuration of a system and when they were fetched.
    ///
    /// # Errors
    /// Returns an error if the login or the request fails.
    pub fn system(
        &mut self,
        system_id: &str,
        control_identifier: ControlIdentifier,
        source: Source,
    ) -> Result<(System, DateTime<Utc>), Error> {
        self.get(
            &system_url(system_id, control_identifier),
            source,
            TimeDelta::zero(),
        )
    }

    /// Send a change to the API, the cloud takes a few seconds to reflect it.
    ///
    /// # Errors
    /// Returns an error if the login or the request fails.
    pub fn execute(&mut self, command: &Command) -> Result<(), Error> {
        let request = command.request(Local::now());

        log::info!("MyVaillant: {:?} {}", request.method, request.url);

        self.send(&request)?;

        // The cached state is outdated now
        self.cache
            .remove(&system_url(&command.system_id, command.control_identifier));

        Ok(())
    }

    fn get<T: DeserializeOwned>(
        &mut self,
        url: &str,
        source: Source,
        max_age: TimeDelta,
    ) -> Result<(T, DateTime<Utc>), Error> {
        if let Some((body, fetched_at)) = self.cache.get(url)
            && (source == Source::Cache || Utc::now() - fetched_at < max_age)
        {
            return Ok((T::deserialize(body)?, fetched_at));
        }

        if source == Source::Cache {
            return Err(Error::NotCached(url.to_string()));
        }

        let body: serde_json::Value = self
            .send(&Request::get(url.to_string()))?
            .body_mut()
            .read_json()?;

        // Parse first, so responses that can't be read are not cached
        let value = T::deserialize(&body)?;
        self.cache.insert(url, body);

        Ok((value, Utc::now()))
    }

    /// Send the request, logging in again once if the token was rejected.
    fn send(&mut self, request: &Request) -> Result<Response<Body>, Error> {
        if let Some(until) = self.cache.paused_until() {
            return Err(Error::QuotaExceeded(until));
        }

        let result = match self.try_send(request) {
            // The token may be revoked before it expires
            Err(Error::Status { status: 401, .. }) => {
                log::warn!("MyVaillant: token rejected, logging in again");

                self.cache.set_token(None);
                self.try_send(request)
            }
            result => result,
        };

        // Persisted, so restarts don't hit the API while it is paused
        if let Err(Error::QuotaExceeded(until)) = &result {
            self.cache.pause_until(*until);
        }

        result
    }

    fn try_send(&mut self, request: &Request) -> Result<Response<Body>, Error> {
        let access_token = self.access_token()?;
        let url = request.url.as_str();
        let body = || {
            request
                .body
                .clone()
                .unwrap_or_else(|| serde_json::json!({}))
        };

        let mut response = match request.method {
            Method::Get => authorized(self.agent.get(url), &access_token).call(),
            Method::Delete => authorized(self.agent.delete(url), &access_token).call(),
            Method::Post => authorized(self.agent.post(url), &access_token).send_json(body()),
            Method::Patch => authorized(self.agent.patch(url), &access_token).send_json(body()),
        }?;

        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }

        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse::<i64>().ok());
        let body = response.body_mut().read_to_string().unwrap_or_default();

        if status.is_client_error() && is_quota_exceeded(&body) {
            let pause = retry_after
                .map(TimeDelta::seconds)
                .or_else(|| replenished_in(&body))
                .unwrap_or(QUOTA_PAUSE);

            return Err(Error::QuotaExceeded(Local::now() + pause));
        }

        Err(Error::Status {
            status: status.as_u16(),
            url: url.to_string(),
            body,
        })
    }

    fn access_token(&mut self) -> Result<String, Error> {
        let token = match self.cache.token() {
            Some(token) if !token.expires_soon() => return Ok(token.access_token.clone()),
            Some(token) => {
                auth::refresh(&self.agent, &self.credentials.realm, token).or_else(|err| {
                    log::warn!("MyVaillant: token refresh failed, logging in again: {err}");

                    auth::login(&self.agent, &self.credentials)
                })?
            }
            None => {
                log::info!("MyVaillant: logging in");

                auth::login(&self.agent, &self.credentials)?
            }
        };

        let access_token = token.access_token.clone();
        // Persisted, so restarts don't have to log in again
        self.cache.set_token(Some(token));

        Ok(access_token)
    }
}

/// Base URL of a system for reading its state and changing it.
pub(super) fn system_url(system_id: &str, control_identifier: ControlIdentifier) -> String {
    match control_identifier {
        ControlIdentifier::Vrc700 => format!("{VRC700_API_URL_BASE}/systems/{system_id}"),
        _ => format!("{API_URL_BASE}/systems/{system_id}/tli"),
    }
}

/// Add the headers of the myVAILLANT app.
fn authorized<B>(request: RequestBuilder<B>, access_token: &str) -> RequestBuilder<B> {
    request
        .header("Authorization", format!("Bearer {access_token}"))
        .header("x-app-identifier", "VAILLANT")
        .header("Accept-Language", "en-GB")
        .header("Accept", "application/json, text/plain, */*")
        .header("x-client-locale", "en-GB")
        .header("x-idm-identifier", "KEYCLOAK")
        .header("ocp-apim-subscription-key", SUBSCRIPTION_KEY)
        .header("User-Agent", "okhttp/4.9.2")
}

/// Azure API Management answers with "Quota exceeded" or "Out of call volume quota"
fn is_quota_exceeded(body: &str) -> bool {
    let body = body.to_lowercase();

    body.contains("quota exceeded") || body.contains("out of call volume quota")
}

/// Parse "Quota will be replenished in HH:MM:SS" from the response body.
fn replenished_in(body: &str) -> Option<TimeDelta> {
    let (_, rest) = body.split_once("Quota will be replenished in ")?;
    let mut parts = rest
        .get(..8)?
        .splitn(3, ':')
        .map(|part| part.parse::<i64>().ok());
    let (hours, minutes, seconds) = (parts.next()??, parts.next()??, parts.next()??);

    Some(TimeDelta::seconds(hours * 3600 + minutes * 60 + seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quota_replenish_time() {
        assert!(is_quota_exceeded(
            r#"{"statusCode":403,"message":"Out of call volume quota. Quota will be replenished in 02:13:07."}"#
        ));
        assert_eq!(
            replenished_in("Out of call volume quota. Quota will be replenished in 02:13:07."),
            Some(TimeDelta::seconds(2 * 3600 + 13 * 60 + 7))
        );
        assert_eq!(replenished_in("Quota exceeded"), None);
    }
}
