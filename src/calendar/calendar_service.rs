use std::collections::BTreeSet;

use chrono::{Datelike as _, NaiveDate};
use url::Url;
use uuid::Uuid;

use crate::icalendar::{Calendar, CalendarMap, CalendarMapChange, CalendarMapExt, Event};

use super::{caldav, filter_time_range, request_event, CaldavClient, Credentials, GRID_LENGTH};

#[derive(Debug)]
pub struct CalendarService {
  client: CaldavClient,
  calendar: CalendarMap,
  pub filtered_calendars: BTreeSet<Uuid>,
}

impl CalendarService {
  pub fn new(credentials: Credentials, url: Url) -> Self {
    Self {
      client: CaldavClient::new(credentials, url),
      calendar: CalendarMap::new(),
      filtered_calendars: BTreeSet::new(),
    }
  }

  pub const fn calendar_map(&self) -> &CalendarMap {
    &self.calendar
  }

  pub fn fetch(client: CaldavClient, date: NaiveDate) -> Result<CalendarMap, caldav::Error> {
    let first_of_month = date.with_day0(0).unwrap();
    let first = first_of_month - chrono::Months::new(6);
    let last = first_of_month + chrono::Months::new(6);
    let request = request_event(filter_time_range(first, last));

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

  pub const fn client(&self) -> &CaldavClient {
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

  /// Generates a grid of events for the given date.
  /// The grid is a list of events for each day in the month.
  /// Each event is a tuple of the end index and the event.
  /// Start and end index are capped to the size of the grid.
  pub fn generate_grid(&self, (first, last): (NaiveDate, NaiveDate)) -> [Vec<&Event>; GRID_LENGTH] {
    let mut grid = [(); GRID_LENGTH].map(|_| Vec::new());

    for (_, calendar, _, event) in self.calendar.flat_iter() {
      let event_start = event.start_date();
      let event_end = event.end_date();

      if event_end < event_start {
        log::error!(
          "[{}] Event end is before start: {:?}",
          calendar.name,
          event,
        );
        continue;
      }

      if event_end <= first || event_start > last {
        continue;
      }

      let start_clamped = (event_start).clamp(first, last);
      let idx: usize = (start_clamped - first).num_days() as usize;

      grid[idx].push(event)
    }

    grid
  }
}
