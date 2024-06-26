use std::rc::Rc;

use chrono::{DateTime, Datelike, Days, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use url::Url;
use uuid::Uuid;

pub const EVENT_DEFAULT_COLOR: &str = "#deb887";
pub const EARLIEST_NAIVE_TIME: NaiveTime = NaiveTime::from_hms(0, 0, 0);
pub const LASTEST_NAIVE_TIME: NaiveTime = NaiveTime::from_hms(23, 59, 59);

// @TODO use more effecient ways to send events through components
pub type EventBox = Rc<Event>;

#[derive(Debug, Clone)]
pub struct Event {
  pub etag: String,
  pub uid: Uuid,
  pub calendar_uid: Uuid,
  pub summary: String,
  pub description: Option<String>,
  pub start: NaiveDateTime,
  pub end: NaiveDateTime,
  pub color: Option<String>,
  pub url: Url,
}

impl Event {
  pub fn description(&self) -> &str {
    self.description.as_deref().unwrap_or_default()
  }

  pub fn color(&self) -> &str {
    self.color.as_deref().unwrap_or("#deb887")
  }

  pub fn fg_color(&self) -> &str {
    fg_from_bg_w3c(self.color()).unwrap_or("#000000")
  }

  pub fn tooltip(&self) -> String {
    format!(
      "{}\n{} - {}",
      self.summary,
      format_date(&self.start_tz()),
      format_date(&self.end_tz()),
    )
  }

  pub const fn start_date(&self) -> NaiveDate {
    self.start.date()
  }

  pub fn start_tz(&self) -> DateTime<chrono_tz::Tz> {
    self.start.and_utc().with_timezone(&chrono_tz::Europe::Berlin)
  }

  pub fn end_date(&self) -> NaiveDate {
    if self.end.hour() == 0 && self.end.minute() == 0 {
      self.end.date() - Days::new(1)
    } else {
      self.end.date()
    }
  }

  pub fn end_tz(&self) -> DateTime<chrono_tz::Tz> {
    self.end.and_utc().with_timezone(&chrono_tz::Europe::Berlin)
  }

  pub fn start_end_dates(&self) -> (NaiveDate, NaiveDate)  {
    (self.start_date(), self.end_date())
  }

  pub fn is_between_dates(&self, start: NaiveDate, end: NaiveDate) -> bool {
    let start_date_time = start.and_time(EARLIEST_NAIVE_TIME);
    let end_date_time = end.and_time(LASTEST_NAIVE_TIME);

    self.start <= end_date_time && self.end > start_date_time
  }

  pub fn days_between_dates(&self, start: NaiveDate, end: NaiveDate) -> i64 {
    let start_date_time = start.and_time(EARLIEST_NAIVE_TIME);
    let end_date_time = (end + Days::new(1)).and_time(EARLIEST_NAIVE_TIME);

    (self.end.clamp(start_date_time, end_date_time) - self.start.clamp(start_date_time, end_date_time)).num_days()
  }

  pub fn delta_text(&self, date: NaiveDateTime) -> String {
    if date >= self.start && date <= self.end {
      return "jetzt".to_string();
    }

    let delta = if date > self.end {
      self.end - date
    } else {
      self.start - date
    };

    let days = delta.num_days();
    let hours = delta.num_hours();
    let minutes = delta.num_minutes();

    if minutes == -1 {
      format!("vor {} Minute", minutes.abs())
    } else if minutes == 1 {
      format!("in {minutes} Minute")
    } else if hours == -1 {
      format!("vor {} Stunde", hours.abs())
    } else if hours == 1 {
      format!("in {hours} Stunde")
    } else if days == -1 {
      format!("vor {} Tag", days.abs())
    } else if days == 1 {
      format!("in {days} Tag")
    } else if days < 0 {
      format!("vor {} Tagen", days.abs())
    } else if hours < 0 {
      format!("vor {} Stunden", hours.abs())
    } else if minutes < 0 {
      format!("vor {} Minuten", minutes.abs())
    } else if days > 0 {
      format!("in {days} Tagen")
    } else if hours > 0 {
      format!("in {hours} Stunden")
    } else if minutes > 0 {
      format!("in {minutes} Minuten")
    } else {
      "jetzt".to_string()
    }
  }

  pub fn all_matching_between(&self, start: NaiveDate, end: NaiveDate) -> impl Iterator<Item = NaiveDate> {
    let clamped_start = self.start_date().clamp(start, end);
    let clamped_end = self.end_date().clamp(start, end);

    clamped_start.iter_days().take_while(move |date| date <= &clamped_end)
  }
}

impl PartialEq for Event {
  fn eq(&self, other: &Self) -> bool {
    self.uid == other.uid && self.etag == other.etag
  }
}

fn fg_from_bg_w3c<'a>(bg_color: &str) -> Option<&'a str> {
  let color = if bg_color.starts_with('#') { &bg_color[1..bg_color.len()] } else { bg_color };

  let mut rgb = [
    f32::from(i16::from_str_radix(&color[0..2], 16).ok()?) / 255.0,
    f32::from(i16::from_str_radix(&color[2..4], 16).ok()?) / 255.0,
    f32::from(i16::from_str_radix(&color[4..6], 16).ok()?) / 255.0,
  ];

   rgb = rgb.map(|c| {
    if c <= 0.04045 {
      c / 12.92
    } else {
      ((c + 0.055) / 1.055).powf(2.4)
    }
  });

  if rgb[0].mul_add(0.2126, rgb[1].mul_add(0.7152, rgb[2] * 0.0722)) > 0.179 {
    Some("#000000")
  } else {
    Some("#ffffff")
  }
}

fn format_date(date: &DateTime<chrono_tz::Tz>) -> String {
  let now = chrono::Utc::now().with_timezone(&date.timezone());

  let formatted = if date.year() == now.year() {
    date.format_localized("%m. %b %H:%M", chrono::Locale::de_DE)
  } else {
    date.format_localized("%m. %b %Y %H:%M", chrono::Locale::de_DE)
  };

  formatted.to_string()
}
