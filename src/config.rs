use std::path::PathBuf;

use url::Url;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Config {
  pub ical: Ical,
  pub videos: Option<Vec<Video>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Ical {
  pub url: Url,
  pub username: String,
  pub password_file: Option<PathBuf>,
  pub password: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Video {
  pub name: String,
  pub url: Url,
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

pub fn init(path: PathBuf) -> Result<Config, Box<dyn std::error::Error>> {
  let string = std::fs::read_to_string(path)?;
  let mut config: Config = toml::from_str(&string)?;

  if let Some(file) = &config.ical.password_file {
    let password = std::fs::read_to_string(file)?;

    config.ical.password = Some(password);
  }

  Ok(config)
}
