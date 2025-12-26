use std::str::FromStr;

use chrono::{NaiveDateTime, NaiveTime, TimeZone};
use chrono_tz::Tz;
use icalendar::{CalendarDateTime, Component as _, DatePerhapsTime};
use rrule::{RRule, RRuleError, Unvalidated};
use url::Url;
use uuid::Uuid;

use super::{event::Event, extract};

#[derive(Debug)]
pub enum EventBuilderError {
    NoEtag,
    InvalidColor,
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
    pub summary: Option<String>,
    pub description: Option<String>,
    pub start: Option<DatePerhapsTime>,
    pub end: Option<DatePerhapsTime>,
    pub url: Option<String>,
    pub rrule: Option<String>,
}

impl EventBuilder {
    /// Builds the event.
    ///
    /// # Errors
    /// Returns an error if the required fields are missing or invalid.
    pub fn build(self) -> Result<Event, EventBuilderError> {
        let etag = self.etag.ok_or(EventBuilderError::NoEtag)?;
        let uid_str = self.uid.ok_or(EventBuilderError::NoUid)?;
        let uid: Uuid = Uuid::parse_str(&uid_str)
            .map_err(|err| EventBuilderError::InvalidUid(err.to_string()))?;
        let summary = self.summary.unwrap_or_else(|| "<kein Titel>".to_owned());
        let start = self.start.ok_or(EventBuilderError::NoStart)?;
        let start = date_perhaps_time_to_date_time(start).ok_or(EventBuilderError::InvalidStart)?;
        let end = self.end.ok_or(EventBuilderError::NoEnd)?;
        let end = date_perhaps_time_to_date_time(end).ok_or(EventBuilderError::InvalidStart)?;
        let url_str = self.url.ok_or(EventBuilderError::NoUrl)?;
        let url =
            Url::parse(&url_str).map_err(|err| EventBuilderError::InvalidUrl(err.to_string()))?;
        let rrule = match self.rrule {
            Some(rrule) => Some(
                rrule
                    .parse::<RRule<Unvalidated>>()
                    .map_err(EventBuilderError::InvalidRRule)?,
            ),
            None => None,
        };

        Ok(Event {
            etag,
            uid,
            summary,
            description: self.description,
            start,
            end,
            url,
            rrule,
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

    pub fn set_url_opt(mut self, url: Option<String>) -> Self {
        self.url = url;
        self
    }

    pub fn set_rrule_opt(mut self, rrule: Option<String>) -> Self {
        self.rrule = rrule;
        self
    }

    pub fn with_base_url(mut self, base_url: &Url) -> Self {
        self.url = self.url.map_or_else(
            || None,
            |url| base_url.join(&url).ok().map(|url| url.as_str().to_owned()),
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
            .set_rrule_opt(
                event
                    .property_value("RRULE")
                    .map(std::borrow::ToOwned::to_owned),
            )
    }
}

impl From<&xmltree::Element> for EventBuilder {
    fn from(element: &xmltree::Element) -> Self {
        let data = extract::event_data(element).unwrap_or_default();

        icalendar::parser::read_calendar(&data)
            .ok()
            .map(icalendar::Calendar::from)
            .and_then(|calendar| {
                calendar.components.into_iter().find_map(|c| match c {
                    icalendar::CalendarComponent::Event(event) => Some(Self::from(&event)),
                    _ => None,
                })
            })
            .unwrap_or_default()
            .set_url_opt(extract::href(element))
            .set_etag_opt(extract::etag(element))
    }
}

fn date_perhaps_time_to_date_time(date: DatePerhapsTime) -> Option<NaiveDateTime> {
    Some(match date {
        DatePerhapsTime::DateTime(dt) => match dt {
            CalendarDateTime::Floating(dt) => dt,
            CalendarDateTime::WithTimezone { date_time, tzid } => Tz::from_str(&tzid)
                .ok()?
                .from_local_datetime(&date_time)
                .single()?
                .naive_utc(),
            CalendarDateTime::Utc(dt) => dt.naive_utc(),
        },
        DatePerhapsTime::Date(dt) => dt.and_time(NaiveTime::default()),
    })
}
