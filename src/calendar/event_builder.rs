use std::str::FromStr;

use chrono::{DateTime, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use icalendar::{CalendarDateTime, Component as _, DatePerhapsTime};
use url::Url;
use uuid::Uuid;
use rrule::{RRule, RRuleError, Unvalidated};

use super::event_uuid::EventUuid;
use super::{extract, Event};

#[derive(Debug)]
pub enum EventBuilderError {
  NoEtag,
  NoUid,
  NoCalendarUid,
  InvalidUid(String),
  NoSummary,
  NoStart,
  InvalidStart,
  NoEnd,
  InvalidEnd,
  NoUrl,
  InvalidUrl(String),
  InvalidRRule(RRuleError),
}

#[derive(Debug, Default)]
pub struct EventBuilder {
  pub etag: Option<String>,
  pub uid: Option<String>,
  pub calendar_uid: Option<Uuid>,
  pub summary: Option<String>,
  pub description: Option<String>,
  pub start: Option<DatePerhapsTime>,
  pub end: Option<DatePerhapsTime>,
  pub color: Option<String>,
  pub url: Option<String>,
  pub rrule: Option<String>,
}

impl EventBuilder {
  /// Builds the event.
  ///
  /// # Errors
  /// Returns an error if the required fields are missing or invalid.
  pub fn build(self) -> Result<Vec<Event>, EventBuilderError> {
    let etag = self.etag.ok_or(EventBuilderError::NoEtag)?;
    let uid_str = self.uid.ok_or(EventBuilderError::NoUid)?;
    let uid: Uuid = Uuid::parse_str(&uid_str).map_err(|err| EventBuilderError::InvalidUid(err.to_string()))?;
    let calendar_uid = self.calendar_uid.ok_or(EventBuilderError::NoCalendarUid)?;
    let summary = self.summary.unwrap_or_else(|| "<kein Titel>".to_owned());
    let start = self.start.ok_or(EventBuilderError::NoStart)?;
    let start = date_perhaps_time_to_date_time(start).ok_or(EventBuilderError::InvalidStart)?;
    let end = self.end.ok_or(EventBuilderError::NoEnd)?;
    let end = date_perhaps_time_to_date_time(end).ok_or(EventBuilderError::InvalidStart)?;
    let url_str = self.url.ok_or(EventBuilderError::NoUrl)?;
    let url = Url::parse(&url_str).map_err(|err| EventBuilderError::InvalidUrl(err.to_string()))?;

    let create = move |start: NaiveDateTime, end: NaiveDateTime, idx: Option<usize>| Event {
      etag,
      uid: EventUuid::new(uid, idx),
      calendar_uid,
      summary,
      description: self.description,
      start,
      end,
      color: self.color,
      url,
    };

    let list = if let Some(rule) = self.rrule {
      let rrule = rule.parse::<RRule<Unvalidated>>().map_err(EventBuilderError::InvalidRRule)?;

      let end_dates = rrule.clone().build(end.with_timezone(&rrule::Tz::UTC)).map_err(EventBuilderError::InvalidRRule)?;
      let start_dates = rrule.build(start.with_timezone(&rrule::Tz::UTC)).map_err(EventBuilderError::InvalidRRule)?;

      start_dates
        .into_iter()
        .zip(end_dates.into_iter())
        .map(|(start, end)| (start.naive_utc(), end.naive_utc()))
        .enumerate()
        .map(|(idx, (start, end))| create.clone()(start, end, Some(idx)))
        .collect()
    } else {
      vec![create(start.naive_utc(), end.naive_utc(), None)]
    };

    Ok(list)
  }

  pub fn set_etag_opt(mut self, etag: Option<String>) -> Self {
    self.etag = etag;
    self
  }

  pub fn set_uid_opt(mut self, uid: Option<String>) -> Self {
    self.uid = uid;
    self
  }

  pub const fn set_calendar_uid_opt(mut self, calendar_uid: Option<Uuid>) -> Self {
    self.calendar_uid = calendar_uid;
    self
  }

  pub fn set_summary_opt(mut self, summary: Option<String>) -> Self {
    self.summary = summary;
    self
  }

  pub fn set_description_opt(mut self, description: Option<String>) -> Self {
    self.description = description;
    self
  }

  pub fn set_start_opt(mut self, start: Option<DatePerhapsTime>) -> Self {
    self.start = start;
    self
  }

  pub fn set_end_opt(mut self, end: Option<DatePerhapsTime>) -> Self {
    self.end = end;
    self
  }

  pub fn set_color_opt(mut self, color: Option<String>) -> Self {
    self.color = color;
    self
  }

  pub fn set_url_opt(mut self, url: Option<String>) -> Self {
    self.url = url;
    self
  }

  pub fn set_rrule_opt(mut self, rrule: Option<String>) -> Self {
    self.rrule = rrule;
    self
  }

  pub fn with_default_color(mut self, color: &str) -> Self {
    if self.color.is_none() {
      self.color = Some(color.to_string());
    }
    self
  }

  pub fn with_base_url(mut self, base_url: &Url) -> Self {
    self.url = self.url.map_or_else(
      || None,
      |url| base_url.join(&url).ok().map(|url| url.as_str().to_owned())
    );
    self
  }
}

impl From<&icalendar::Event> for EventBuilder {
  fn from(event: &icalendar::Event) -> Self {
    Self::default()
      .set_summary_opt(event.get_summary().map(std::borrow::ToOwned::to_owned))
      .set_description_opt(event.get_description().map(std::borrow::ToOwned::to_owned))
      .set_start_opt(event.get_start())
      .set_end_opt(event.get_end())
      .set_uid_opt(event.get_uid().map(std::borrow::ToOwned::to_owned))
      .set_rrule_opt(event.property_value("RRULE").map(std::borrow::ToOwned::to_owned))
  }
}

impl From<&xmltree::Element> for EventBuilder {
  fn from(element: &xmltree::Element) -> Self {
    let data = extract::event_data(element).unwrap_or_default();

    icalendar::parser::read_calendar(&data).ok()
      .map(icalendar::Calendar::from)
      .and_then(|calendar| calendar.components.into_iter().find_map(|c| match c {
        icalendar::CalendarComponent::Event(event) => Some(Self::from(&event)),
        _ => None,
      }))
      .unwrap_or_default()
      .set_url_opt(extract::href(element))
      .set_etag_opt(extract::etag(element))
  }
}

fn date_perhaps_time_to_date_time(date: DatePerhapsTime) -> Option<DateTime<Utc>> {
  Some(match date {
    DatePerhapsTime::DateTime(dt) => match dt {
      CalendarDateTime::Floating(dt) => dt.and_utc(),
      CalendarDateTime::WithTimezone { date_time, tzid } => Tz::from_str(&tzid).ok()?.from_local_datetime(&date_time).single()?.with_timezone(&Utc),
      CalendarDateTime::Utc(dt) => dt,
    },
    DatePerhapsTime::Date(dt) => dt.and_time(NaiveTime::default()).and_utc(),
  })
}
