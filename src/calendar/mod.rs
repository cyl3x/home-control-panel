use chrono::NaiveDate;
use url::Url;
use icalendar::Event;


mod date_manager;
mod calendar_manager;
mod caldav;
pub use date_manager::*;
pub use caldav::*;

use crate::config::Config;

use self::calendar_manager::CalendarManager;

pub const CACHE_DIR: &str = "/tmp/dav-cache";

pub const EVENT_COLOR: &str = "calendar_color";


/// Initializes a Provider, and run an initial sync from the server
pub fn init(config: &Config) -> CalDavProvider {
  let credentials = Credentials::Basic(config.ical.username.clone(), config.ical.password.as_ref().unwrap().clone());

  let mut provider = CalDavProvider::new(credentials, config.ical.url.clone());

  provider.sync();

  provider
}

#[derive(Debug)]
pub struct CalDavProvider {
  date_manager: DateManager,
  calendar_manager: CalendarManager,
}

impl CalDavProvider {
  pub fn new(credentials: Credentials, url: Url) -> Self {
    Self {
      date_manager: DateManager::new(chrono::Utc::now().naive_utc().date()),
      calendar_manager: CalendarManager::new(credentials, url),
    }
  }

  pub const fn date(&self, idx: usize) -> NaiveDate {
    self.date_manager.date(idx)
  }

  pub const fn selected(&self) -> NaiveDate {
    self.date_manager.current()
  }

  pub const fn selected_idx(&self) -> usize {
    self.date_manager.current_idx()
  }

  /// Set the date of the calendar
  /// Returns the index of the date in the month grid, if the month has changed
  pub fn select(&mut self, date: NaiveDate) -> Option<usize> {
    self.date_manager.set_date(date)
  }

  pub fn select_idx(&mut self, idx: usize) -> Option<usize> {
    self.select(self.date_manager.date(idx))
  }

  pub fn date_row(&self, row_idx: usize) -> [NaiveDate; GRID_COLS] {
    self.date_manager.row(row_idx)
  }

  pub fn next_month(&mut self) -> NaiveDate {
    self.date_manager.next_month()
  }

  pub fn prev_month(&mut self) -> NaiveDate {
    self.date_manager.prev_month()
  }

  pub fn sync(&mut self) {
    self.calendar_manager.sync(self.date_manager.current());
  }

  pub fn calendar_grid(&self) -> [Vec<(usize, &Event)>; GRID_LENGTH] {
    self.calendar_manager.generate_grid(&self.date_manager)
  }

  pub const fn date_grid(&self) -> &[NaiveDate; GRID_LENGTH] {
    self.date_manager.month_grid()
  }
}
