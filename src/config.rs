use std::path::PathBuf;

use url::Url;
use uuid::Uuid;

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    pub ical: Ical,
    #[serde(default)]
    pub videos: Vec<Video>,
    #[serde(default)]
    pub calendar: Calendars,
    #[serde(default)]
    pub screensaver: Screensaver,
}

#[derive(serde::Deserialize)]
pub struct Ical {
    pub url: Url,
    pub username: String,
    pub password_file: Option<PathBuf>,
    pub password: Option<String>,
}

impl core::fmt::Debug for Ical {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("IcalUrl")
            .field("url", &self.url)
            .field("username", &self.username)
            .field("password_file", &self.password_file)
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
    pub upcomming: Option<UpcommingFilter>,
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
pub struct UpcommingFilter {
    #[serde(default)]
    pub exclude: Vec<Uuid>,
    #[serde(default)]
    pub include: Vec<Uuid>,
    #[serde(default)]
    pub skip_oneliner: Vec<Uuid>,
}

impl UuidFilter {
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
    pub timeout: u64,
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

#[derive(Clone, Debug, serde::Deserialize)]
pub struct StartEndTimes {
    pub start: chrono::NaiveTime,
    pub end: chrono::NaiveTime,
}

pub fn init(path: PathBuf) -> Result<Config, Box<dyn std::error::Error>> {
    let string = std::fs::read_to_string(path)?;
    let mut config: Config = toml::from_str(&string)?;

    if let Some(file) = &config.ical.password_file {
        let password = std::fs::read_to_string(file)?;

        config.ical.password = Some(password);
    }

    Ok(config)
}

const fn default_screensaver_timeout() -> u64 {
    600
}
