use std::collections::BTreeSet;

use chrono::{Datelike as _, NaiveDate};
use url::Url;
use uuid::Uuid;

use crate::icalendar::{Calendar, CalendarMap, CalendarMapChange, CalendarMapExt, Event};

use super::{caldav, filter_time_range, request_event, Client, Credentials};

#[derive(Debug)]
pub struct CalendarService {
  client: Client,
  calendar: CalendarMap,
  pub filtered_calendars: BTreeSet<Uuid>,
}

impl CalendarService {
  pub fn new(credentials: Credentials, url: Url) -> Self {
    Self {
      client: Client::new(credentials, url),
      calendar: CalendarMap::new(),
      filtered_calendars: BTreeSet::new(),
    }
  }

  pub const fn calendar_map(&self) -> &CalendarMap {
    &self.calendar
  }

  /// Fetches the events around the given date.
  /// The range is from 6 months before and after the given date.
  /// The events are grouped by calendar.
  ///
  /// # Errors
  /// Returns an error if the request or parsing fails.
  pub fn fetch(client: Client, date: NaiveDate) -> Result<CalendarMap, caldav::Error> {
    let first_of_month = date.with_day0(0).unwrap();
    let first = first_of_month - chrono::Months::new(6);
    let last = first_of_month + chrono::Months::new(6);
    let request = request_event(&filter_time_range(first, last));

    client
      .get_calendars()?
      .into_iter()
      .map(|(cal_uid, calendar)| {
        match client.get_events(&request, &calendar) {
          Ok(events) => Ok((cal_uid, (calendar, events))),
          Err(e) => Err(e),
        }
      })
      .collect::<Result<_, _>>()
  }

  pub fn apply_map(&mut self, map: CalendarMap) -> impl Iterator<Item = (Uuid, Uuid, CalendarMapChange)> + '_ {
    self.calendar.exchange(map)
      .map(|(cal_uid, event_uid, event_change)| {
        if self.filtered_calendars.contains(&cal_uid) {
          (cal_uid, event_uid, CalendarMapChange::Removed(event_change.into_inner()))
        } else {
          (cal_uid, event_uid, event_change)
        }
      })
  }

  pub const fn client(&self) -> &Client {
    &self.client
  }

  pub fn events(&self) -> impl Iterator<Item = &Event> {
    self.calendar.flat_events()
  }

  pub fn calendars(&self) -> impl Iterator<Item = &Calendar> {
    self.calendar.flat_calendars()
  }

  pub fn is_filtered(&self, uid: &Uuid) -> bool {
    self.filtered_calendars.contains(uid)
  }

  pub fn toggle_calendar_filter(&mut self, uid: Uuid, active: bool) {
    if active {
      self.filtered_calendars.remove(&uid);
    } else {
      self.filtered_calendars.insert(uid);
    }
  }
}
