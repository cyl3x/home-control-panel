use chrono::{DateTime, Days, NaiveDate, NaiveTime, Utc};
use url::Url;
use uuid::Uuid;

pub const EVENT_DEFAULT_COLOR: &str = "#deb887";

pub const EARLIEST_NAIVE_TIME: NaiveTime = NaiveTime::from_hms(0, 0, 0);
pub const LASTEST_NAIVE_TIME: NaiveTime = NaiveTime::from_hms(23, 59, 59);

#[derive(Debug, Clone)]
pub struct Event {
  pub etag: String,
  pub uid: Uuid,
  pub calendar_uid: Uuid,
  pub summary: String,
  pub description: Option<String>,
  pub start: DateTime<Utc>,
  pub end: DateTime<Utc>,
  pub color: Option<String>,
  pub url: Url,
}

impl Event {
  pub fn start_date(&self) -> NaiveDate {
    self.start.date_naive()
  }

  pub fn end_date(&self) -> NaiveDate {
    self.end.date_naive()
  }

  pub fn start_end_dates(&self) -> (NaiveDate, NaiveDate)  {
    (self.start_date(), self.end_date())
  }

  pub fn is_between_dates(&self, start: NaiveDate, end: NaiveDate) -> bool {
    let start_date_time = start.and_time(EARLIEST_NAIVE_TIME).and_utc();
    let end_date_time = end.and_time(LASTEST_NAIVE_TIME).and_utc();

    self.start <= end_date_time && self.end > start_date_time
  }

  pub fn days_between_dates(&self, start: NaiveDate, end: NaiveDate) -> i64 {
    let start_date_time = start.and_time(EARLIEST_NAIVE_TIME).and_utc();
    let end_date_time = (end + Days::new(1)).and_time(EARLIEST_NAIVE_TIME).and_utc();

    (self.end.clamp(start_date_time, end_date_time) - self.start.clamp(start_date_time, end_date_time)).num_days()
  }
}

impl PartialEq for Event {
  fn eq(&self, other: &Self) -> bool {
    self.uid == other.uid && self.etag == other.etag
  }
}
