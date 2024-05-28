use anyhow::anyhow;
use env_logger::fmt::Formatter;
use log::kv::{Key, Source, ToKey, Value};
use log::Record;
use std::fmt::Debug;
use std::io::Write;

pub const SOURCE: &str = "source";

pub fn init() {
  env_logger::builder()
  .init();
}

pub trait LogExt<T> {
  fn log_trace(self, snippet: &str) -> anyhow::Result<T>;
  fn log_debug(self, snippet: &str) -> anyhow::Result<T>;
  fn log_info(self, snippet: &str) -> anyhow::Result<T>;
  fn log_warn(self, snippet: &str) -> anyhow::Result<T>;
  fn log_error(self, snippet: &str) -> anyhow::Result<T>;
}

impl<T> LogExt<T> for Option<T> {
  fn log_trace(self, msg: &str) -> anyhow::Result<T> {
    if self.is_none() {
      let location = std::panic::Location::caller().to_string();
      log::trace!(location; "{msg}");
      anyhow::bail!(msg.to_string())
    }

    Ok(self.unwrap())
  }

  fn log_debug(self, msg: &str) -> anyhow::Result<T> {
    if self.is_none() {
      let location = std::panic::Location::caller().to_string();
      log::debug!(location; "{msg}");
      anyhow::bail!(msg.to_string())
    }

    Ok(self.unwrap())
  }

  fn log_info(self, msg: &str) -> anyhow::Result<T> {
    if self.is_none() {
      let location = std::panic::Location::caller().to_string();
      log::info!(location; "{msg}");
      anyhow::bail!(msg.to_string())
    }

    Ok(self.unwrap())
  }

  fn log_warn(self, msg: &str) -> anyhow::Result<T> {
    if self.is_none() {
      let location = std::panic::Location::caller().to_string();
      log::warn!(location; "{msg}");
      anyhow::bail!(msg.to_string())
    }

    Ok(self.unwrap())
  }

  fn log_error(self, msg: &str) -> anyhow::Result<T> {
    if self.is_none() {
      let location = std::panic::Location::caller().to_string();
      log::error!(location; "{msg}");
      anyhow::bail!(msg.to_string())
    }

    Ok(self.unwrap())
  }
}

impl<T, E> LogExt<T> for Result<T, E> where E: std::fmt::Debug {
  fn log_trace(self, msg: &str) -> anyhow::Result<T> {
    if let Err(error) = &self {
      let location = std::panic::Location::caller().to_string();

      log::trace!("[{location}] {msg}: {error:?}");

      anyhow::bail!("{msg}: {error:?}");
    }

    Ok(self.unwrap())
  }

  fn log_debug(self, msg: &str) -> anyhow::Result<T> {
    if let Err(error) = &self {
      let location = std::panic::Location::caller().to_string();

      log::debug!("[{location}] {msg}: {error:?}");

      anyhow::bail!("{msg}: {error:?}");
    }

    Ok(self.unwrap())
  }

  fn log_info(self, msg: &str) -> anyhow::Result<T> {
    if let Err(error) = &self {
      let location = std::panic::Location::caller().to_string();

      log::info!("[{location}] {msg}: {error:?}");

      anyhow::bail!("{msg}: {error:?}");
    }

    Ok(self.unwrap())
  }

  fn log_warn(self, msg: &str) -> anyhow::Result<T> {
    if let Err(error) = &self {
      let location = std::panic::Location::caller().to_string();

      log::warn!("[{location}] {msg}: {error:?}");

      anyhow::bail!("{msg}: {error:?}");
    }

    Ok(self.unwrap())
  }

  fn log_error(self, msg: &str) -> anyhow::Result<T> {
    if let Err(error) = &self {
      let location = std::panic::Location::caller().to_string();

      log::error!("[{location}] {msg}: {error:?}");

      anyhow::bail!("{msg}: {error:?}");
    }

    Ok(self.unwrap())
  }
}
