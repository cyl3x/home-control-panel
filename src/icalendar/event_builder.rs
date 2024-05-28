use icalendar::{Component as _, DatePerhapsTime};
use url::Url;
use uuid::Uuid;

use super::{extract, Event};

#[derive(Debug)]
pub enum EventBuilderError {
  NoEtag,
  NoUid,
  NoCalendarUid,
  InvalidUid(String),
  NoSummary,
  NoStart,
  NoEnd,
  NoUrl,
  InvalidUrl(String),
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
}

impl EventBuilder {
  pub fn build(self) -> Result<Event, EventBuilderError> {
    let etag = self.etag.ok_or(EventBuilderError::NoEtag)?;
    let uid_str = self.uid.ok_or(EventBuilderError::NoUid)?;
    let uid: Uuid = Uuid::parse_str(&uid_str).map_err(|err| EventBuilderError::InvalidUid(err.to_string()))?;
    let calendar_uid = self.calendar_uid.ok_or(EventBuilderError::NoCalendarUid)?;
    let summary = self.summary.ok_or(EventBuilderError::NoSummary)?;
    let start = self.start.ok_or(EventBuilderError::NoStart)?;
    let end = self.end.ok_or(EventBuilderError::NoEnd)?;
    let url_str = self.url.ok_or(EventBuilderError::NoUrl)?;
    let url = Url::parse(&url_str).map_err(|err| EventBuilderError::InvalidUrl(err.to_string()))?;

    Ok(Event {
      etag,
      uid,
      calendar_uid,
      summary,
      description: self.description,
      start,
      end,
      color: self.color,
      url,
    })
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
      .set_summary_opt(event.get_summary().map(|s| s.to_owned()))
      .set_description_opt(event.get_description().map(|s| s.to_owned()))
      .set_start_opt(event.get_start())
      .set_end_opt(event.get_end())
      .set_uid_opt(event.get_uid().map(|s| s.to_owned()))
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