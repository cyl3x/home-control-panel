use chrono::{Datelike as _, NaiveDate};
use url::Url;
use uuid::Uuid;

use crate::icalendar::{CalendarMap, CalendarMapChange, CalendarMapExt, Event};

use super::{caldav, filter_time_range, request_event, CaldavClient, Credentials, GRID_LENGTH};

#[derive(Debug)]
pub struct CalendarService {
  client: CaldavClient,
  calendar: CalendarMap,
}

impl CalendarService {
  pub fn new(credentials: Credentials, url: Url) -> Self {
    Self {
      client: CaldavClient::new(credentials, url),
      calendar: CalendarMap::new(),
    }
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
  }

  pub const fn client(&self) -> &CaldavClient {
    &self.client
  }

  pub fn events(&self) -> impl Iterator<Item = &Event> {
    self.calendar.flat_events()
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
