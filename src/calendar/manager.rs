use chrono::{NaiveDate, NaiveDateTime};
use uuid::Uuid;

use crate::config::{self, UuidFilter};

use super::map::CalendarMap;
use super::{Calendar, Event};

use super::caldav::{Client, Credentials};

#[derive(Debug)]
pub struct Manager {
    pub client: Client,
    map: CalendarMap,
}

impl Manager {
    pub fn new(ical: config::Ical) -> Self {
        Self {
            client: Client::new(ical.url.clone(), Credentials::from(ical)),
            map: CalendarMap::default(),
        }
    }

    // Returns true if the given map is different from the current map
    pub fn set_map(&mut self, map: CalendarMap) -> bool {
        let old_map = std::mem::replace(&mut self.map, map);

        old_map != self.map
    }

    // Returns true if the fetched map is different from the current map
    pub fn update(&mut self) -> bool {
        match self.client.get_map() {
            Ok(map) => self.set_map(map),
            Err(e) => {
                log::error!("Error fetching calendar map: {:?}", e);
                false
            }
        }
    }

    pub fn calendars<'a>(
        &'a self,
        filter: Option<&'a config::UuidFilter>,
    ) -> impl Iterator<Item = &(bool, Calendar)> {
        self.map.calendars().values().filter(move |(_, calendar)| {
            filter.map_or(true, |filter| filter.is_included(&calendar.uid))
        })
    }

    pub fn toggle_calendar(&mut self, uid: Uuid) {
        self.map.toggle_calendar(uid);
    }

    pub fn events_between<'a>(
        &'a self,
        start: NaiveDate,
        end: NaiveDate,
        filter: Option<&'a UuidFilter>,
    ) -> impl Iterator<Item = (&Calendar, &NaiveDateTime, &Event)> {
        self.map
            .events_between(start, end)
            .filter(move |(calendar, _, _)| {
                filter.map_or(true, |filter| filter.is_included(&calendar.uid))
            })
    }

    pub fn calendars_between<'a>(
        &'a self,
        start: NaiveDate,
        end: NaiveDate,
        filter: Option<&'a UuidFilter>,
    ) -> impl Iterator<Item = (&NaiveDateTime, &Calendar)> {
        self.map
            .calendars_between(start, end)
            .filter(move |(_, calendar)| {
                filter.map_or(true, |filter| filter.is_included(&calendar.uid))
            })
    }
}
