use icalendar::{CalendarDateTime, DatePerhapsTime};
use url::Url;
use uuid::Uuid;

use super::{UidChangeset, UidMap, UidMapChange};

pub const EVENT_DEFAULT_COLOR: &str = "#deb887";

pub type EventMap = UidMap<Event>;
pub type EventChangeset<'a> = UidChangeset<'a, Event>;
pub type EventChange<'a> = UidMapChange<'a, Event>;

#[derive(Debug, Clone)]
pub struct Event {
  pub etag: String,
  pub uid: Uuid,
  pub calendar_uid: Uuid,
  pub summary: String,
  pub description: Option<String>,
  pub start: DatePerhapsTime,
  pub end: DatePerhapsTime,
  pub color: Option<String>,
  pub url: Url,
}

impl Event {
  pub fn start_date(&self) -> chrono::NaiveDate {
    date_perhaps_time_to_date(&self.start)
  }

  pub fn end_date(&self) -> chrono::NaiveDate {
    date_perhaps_time_to_date(&self.end)
  }

  pub fn start_end_dates(&self) -> (chrono::NaiveDate, chrono::NaiveDate)  {
    (self.start_date(), self.end_date())
  }
}

impl PartialEq for Event {
  fn eq(&self, other: &Self) -> bool {
    self.uid == other.uid && self.etag == other.etag
  }
}

fn date_perhaps_time_to_date(date: &DatePerhapsTime) -> chrono::NaiveDate {
  match date {
    DatePerhapsTime::DateTime(dt) => match dt {
      CalendarDateTime::Floating(dt) => dt.date(),
      CalendarDateTime::WithTimezone { date_time, .. } => date_time.date(),
      CalendarDateTime::Utc(dt) => dt.date_naive(),
    },
    DatePerhapsTime::Date(dt) => *dt,
  }
}