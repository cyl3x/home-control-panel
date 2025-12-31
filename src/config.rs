use std::path::PathBuf;

use serde::Deserialize;
use serde::de::{self, Deserializer};
use url::Url;
use uuid::Uuid;

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Config {
    pub ical: Ical,
    #[serde(default)]
    pub videos: Vec<Video>,
    #[serde(default)]
    pub calendar: Calendars,
    #[serde(default)]
    pub screensaver: Screensaver,
    #[serde(default)]
    pub grafana: Grafana,
}

#[derive(Clone, serde::Deserialize)]
pub struct Ical {
    pub url: Url,
    pub username: String,
    #[serde(deserialize_with = "deserialize_from_file_opt")]
    pub password: String,
}

impl core::fmt::Debug for Ical {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("IcalUrl")
            .field("url", &self.url)
            .field("username", &self.username)
            .field("password", &"<hidden>")
            .finish()
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Video {
    pub name: String,
    pub url: Url,
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct Calendars {
    pub day: Option<UuidFilter>,
    pub days: Option<UuidFilter>,
    pub month: Option<UuidFilter>,
    pub event: Option<UuidFilter>,
    pub ticker: Option<UuidFilter>,
    pub week: Option<UuidFilter>,
    pub upcomming: Option<UpcomingFilter>,
    pub selection: Option<UuidFilter>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct UuidFilter {
    #[serde(default)]
    pub exclude: Vec<Uuid>,
    #[serde(default)]
    pub include: Vec<Uuid>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct UpcomingFilter {
    #[serde(default)]
    pub exclude: Vec<Uuid>,
    #[serde(default)]
    pub include: Vec<Uuid>,
    #[serde(default)]
    pub skip_oneliner: Vec<Uuid>,
}

impl UuidFilter {
    #[must_use]
    pub fn is_included(&self, uid: &Uuid) -> bool {
        if !self.include.is_empty() && self.include.contains(uid) {
            return true;
        }

        if !self.exclude.is_empty() && !self.exclude.contains(uid) {
            return true;
        }

        false
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Screensaver {
    #[serde(default = "default_screensaver_timeout")]
    pub timeout: u16,
    #[serde(default)]
    pub exclude: Vec<StartEndTimes>,
    #[serde(default)]
    pub dim: Vec<StartEndTimes>,
}

impl Default for Screensaver {
    fn default() -> Self {
        Self {
            timeout: default_screensaver_timeout(),
            exclude: Vec::default(),
            dim: Vec::default(),
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, Default)]
pub struct Grafana {
    pub panels: Vec<GrafanaPanel>,
    pub login: Option<GrafanaLogin>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct GrafanaLogin {
    pub url: Url,
    pub username: String,
    #[serde(deserialize_with = "deserialize_from_file_opt")]
    pub password: String,
    #[serde(deserialize_with = "deserialize_from_file_opt")]
    pub js_script: String,
    #[serde(default)]
    pub developer_extras: bool,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct GrafanaPanel {
    pub name: String,
    pub url: Url,
    pub column: u16,
    pub row: u16,
    pub width: u16,
    pub height: u16,
    #[serde(default)]
    pub developer_extras: bool,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct StartEndTimes {
    pub start: chrono::NaiveTime,
    pub end: chrono::NaiveTime,
}

pub fn init(path: PathBuf) -> Result<Config, Box<dyn std::error::Error>> {
    let string = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&string)?;

    Ok(config)
}

const fn default_screensaver_timeout() -> u16 {
    600
}

fn deserialize_from_file_opt<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = <String as Deserialize>::deserialize(deserializer)?;

    if let Some(path) = s.strip_prefix("file:") {
        return std::fs::read_to_string(path).map_err(de::Error::custom);
    }

    Ok(s)
}
