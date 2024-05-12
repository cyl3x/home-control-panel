use std::collections::HashMap;

use chrono::{Datelike as _, NaiveDate};
use url::Url;
use icalendar::{CalendarComponent, CalendarDateTime, Component, DatePerhapsTime, Event};

use super::{filter_time_range, request_event, CaldavClient, CalendarRef, Credentials, Error, DateManager, GRID_LENGTH};


pub const EVENT_COLOR: &str = "calendar_color";
pub const EVENT_DEFAULT_COLOR: &str = "#deb887";

#[derive(Debug)]
pub struct CalendarManager {
  client: CaldavClient,

  calendars: Vec<CalendarRef>,
  events: HashMap<String, Vec<Event>>,
}

impl CalendarManager {
  pub fn new(credentials: Credentials, url: Url) -> Self {
    Self {
      client: CaldavClient::new(credentials, url),
      calendars: Vec::new(),
      events: HashMap::new(),
    }
  }

  pub fn sync(&mut self, date: NaiveDate) {
    self.sync_calendars();
    self.sync_events(date);
  }

  pub fn generate_grid(&self, manager: &DateManager) -> [Vec<(usize, &Event)>; GRID_LENGTH] {
    let mut grid = [(); GRID_LENGTH].map(|_| Vec::new());
    let first = manager.first();
    let last = manager.last();

    for calendar in &self.calendars {
      for event in self.events.get(&calendar.name).unwrap() {
        let event_start = date_perhaps_time_to_date(event.get_start()).expect("Event must have an start date");
        let event_end = date_perhaps_time_to_date(event.get_end()).expect("Event must have an end date");

        if event_end < event_start {
          log::error!("[{}] Event end is before start: {:?}", calendar.name, event);
          continue;
        }

        if event_end <= first || event_start > last {
          continue;
        }

        let start_clamped = (event_start).clamp(first, last);
        let end_clamped = event_end.clamp(first, last);
        let idx: usize = (start_clamped - first).num_days() as usize;
        let end_idx = (end_clamped - first).num_days() as usize;

        grid[idx].push((end_idx, event))
      }
    }

    grid
  }

  fn sync_calendars(&mut self) {
    self.client.get_calendars().map_or_else(
      |error| eprintln!("Error syncing calendars: {:?}", error),
      |calendars| self.calendars = calendars,
    );
  }

  fn sync_events(&mut self, date: NaiveDate) {
    for calendar in &self.calendars {
      match self.get_parsed_events(calendar, date) {
        Err(error) => {
          eprintln!("[{}] Error syncing events: {:?}", calendar.name, error);
        }
        Ok(mut events) => {
          self.events.entry(calendar.name.clone()).or_default().append(&mut events);
        }
      };
    }
  }

  fn get_parsed_events(&self, calendar: &CalendarRef, date: NaiveDate) -> Result<Vec<icalendar::Event>, Error> {
    let first_of_month = date.with_day0(0).unwrap();
    let first = first_of_month.checked_sub_months(chrono::Months::new(6)).unwrap();
    let last = first_of_month.checked_add_months(chrono::Months::new(6)).unwrap();
    let request = request_event(filter_time_range(first, last));
    let cal_color = calendar.color.clone().unwrap_or_else(|| EVENT_DEFAULT_COLOR.into());

    let event_refs = self.client.get_events(request, &calendar.url)?;

    let events = event_refs
      .into_iter()
      .filter_map(|event_ref| 
        match icalendar::parser::read_calendar(&event_ref.data) {
          Ok(calendar) => Some(icalendar::Calendar::from(calendar)),
          Err(error) => {
            log::error!("[{}] Error parsing event: {:?}", calendar.name, error);
            None
          }
        }
      )
      .flat_map(|calendar| 
        calendar.components.into_iter().filter_map(|c| match c {
          CalendarComponent::Event(mut event) => {
            event.add_property(EVENT_COLOR, &cal_color);
            Some(event)
          },
          _ => None,
        })
      )
      .collect();

    Ok(events)
  }

  // pub fn get_events_grid(&self, calendar_ref: &CalendarRef) -> Result<[Vec<(usize, Event)>; GRID_LENGTH], Error> {
  //   let request = request_event(filter_time_range(self.manager.first(), self.manager.last()));

  //   let event_refs = self.client.get_events(request, &calendar_ref.url)?;
  //   let mut grid: [Vec<(usize, Event)>; GRID_LENGTH] = [(); GRID_LENGTH].map(|_| Vec::new());

  //   for event_ref in event_refs {
  //     icalendar::parser::read_calendar(event_ref.data.as_str()).map_or_else(
  //       |error| {
  //         eprintln!("Error parsing event: {:?}", error);
  //       },
  //       |calendar| {
  //         let calendar = icalendar::Calendar::from(calendar);
        
  //         for mut event in calendar.components.into_iter().filter_map(|c| c.as_event().cloned()) {
  //           let event_start = date_perhaps_time_to_date(event.get_start()).expect("Event must have an start date");
  //           let event_end = date_perhaps_time_to_date(event.get_end()).expect("Event must have an end date");
            
  //           let start_clamped = (event_start).clamp(self.manager.first(), self.manager.last());
  //           let end_clamped = event_end.clamp(self.manager.first(), self.manager.last());
  //           let start_idx = (start_clamped - self.manager.first()).num_days() as usize;
  //           let end_idx = (end_clamped - self.manager.first()).num_days() as usize;
            
  //           if event_end < event_start {
  //             eprintln!("Event end is before start: {:?}", event);
  //           }
            
  //           if let Some(color) = &calendar_ref.color {
  //             event.add_property(EVENT_COLOR, color);
  //           }

  //           // println!("Event {:?} from {:?} to {:?}", event.get_summary().unwrap(), event.get_start().unwrap(), event.get_end().unwrap());

  //           grid[start_idx].push((end_idx, event))
  //         }
  //       }
  //     );
  //   }

  //   Ok(grid)
  // }

  // pub fn get_all_events(&self) -> Result<Vec<icalendar::Event>, Error> {
  //   let mut events = Vec::new();
  //   for calendar in &self.calendars {
  //     events.extend_from_slice(&self.get_parsed_events(calendar)?);
  //   }

  //   Ok(events)
  // }

  // pub fn get_all_events_grid(&self) -> Result<[Vec<(usize, Event)>; GRID_LENGTH], Error> {
  //   let mut grid = [(); GRID_LENGTH].map(|_| Vec::new());
  //   for calendar in &self.calendars {
  //     let cal_grid = self.get_events_grid(calendar)?;

  //     for idx in 0..GRID_LENGTH {
  //       grid[idx].extend_from_slice(&cal_grid[idx]);
  //     }
  //   }

  //   Ok(grid)
  // }
}

fn date_perhaps_time_to_date(date: Option<DatePerhapsTime>) -> Option<NaiveDate> {
  match date {
    Some(DatePerhapsTime::DateTime(dt)) => Some(match dt {
      CalendarDateTime::Floating(dt) => dt.date(),
      CalendarDateTime::WithTimezone { date_time, .. } => date_time.date(),
      CalendarDateTime::Utc(dt) => dt.date_naive(),
    }),
    Some(DatePerhapsTime::Date(dt)) => Some(dt),
    _ => None,
  }
}
