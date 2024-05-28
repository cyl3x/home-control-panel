use std::collections::BTreeMap;

use chrono::{Datelike as _, NaiveDate};
use url::Url;
use uuid::Uuid;

use crate::icalendar::{Calendar, Event, EventChange, EventChangeset, EventMap, UidMap};

use super::{caldav, filter_time_range, request_event, CaldavClient, Credentials, GRID_LENGTH};

pub type EventChangeGrid<'a> = [Vec<(usize, &'a EventChange<'a>)>; GRID_LENGTH];
pub type EventGrid<'a> = [Vec<(usize, &'a Event)>; GRID_LENGTH];

#[derive(Debug)]
pub struct CalendarService {
  client: CaldavClient,

  calendars: BTreeMap<Uuid, Calendar>,
  events: EventMap,
}

impl CalendarService {
  pub fn new(credentials: Credentials, url: Url) -> Self {
    Self {
      client: CaldavClient::new(credentials, url),
      calendars: BTreeMap::new(),
      events: UidMap::new(),
    }
  }

  /// Syncs the calendars and events for the given date.
  /// Returns a list of UIDs of events that have changed.
  pub fn sync(&mut self, date: NaiveDate) -> Result<EventChangeset, caldav::Error> {
    self.sync_calendars()?;
    self.sync_events(date)
  }

  /// Generates a grid of events for the given date.
  /// The grid is a list of events for each day in the month.
  /// Each event is a tuple of the end index and the event.
  /// Start and end index are capped to the size of the grid.
  pub fn generate_grid(&self, (first, last): (NaiveDate, NaiveDate)) -> [Vec<&Event>; GRID_LENGTH] {
    let mut grid = [(); GRID_LENGTH].map(|_| Vec::new());

    for (_, _, event) in self.events.flat_iter() {
      let event_start = event.start_date();
      let event_end = event.end_date();

      if event_end < event_start {
        log::error!(
          "[{}] Event end is before start: {:?}",
          self.calendars.get(&event.calendar_uid).unwrap().name,
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

  pub fn generate_grid_from_changeset<'a>((first, last): (NaiveDate, NaiveDate), changeset: &'a EventChangeset) -> [Vec<&'a EventChange<'a>>; GRID_LENGTH] {
    let mut grid = [(); GRID_LENGTH].map(|_| Vec::new());

    let all_between_first_and_last = changeset.flat_iter().filter_map(|(_, _, event)| {
      if event.start_date() >= first && event.end_date() < last {
        Some(event)
      } else {
        None
      }
    });

    for event in all_between_first_and_last {
      let start_clamped = (event.start_date()).clamp(first, last);
      let idx: usize = (start_clamped - first).num_days() as usize;

      grid[idx].push(event)
    }

    grid
  }

  fn sync_calendars(&mut self) -> Result<(), caldav::Error> {
    self.calendars = self.client.get_calendars()?;

    Ok(())
  }

  /// Syncs events for the given date range.
  /// Returns a list of UIDs of events that have changed.
  pub fn sync_events(&mut self, date: NaiveDate) -> Result<EventChangeset, caldav::Error> {
    let first_of_month = date.with_day0(0).unwrap();
    let first = first_of_month.checked_sub_months(chrono::Months::new(6)).unwrap();
    let last = first_of_month.checked_add_months(chrono::Months::new(6)).unwrap();
    let request = request_event(filter_time_range(first, last));

    let mut new_uid_map: EventMap = UidMap::new();
    for (_, calendar) in self.calendars.iter() {
      let new_events = self.client.get_events(&request, calendar)?;
      new_uid_map.merge(new_events);
    }

    Ok(self.events.get_applied_changeset(new_uid_map))
  }
}
