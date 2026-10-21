use chrono::{NaiveDate, NaiveDateTime};
use uuid::Uuid;

use crate::cache;
use crate::config::{self, UuidFilter};

use super::map::CalendarMap;
use super::{Calendar, Event};

use super::caldav::{Client, Credentials};

/// Shows the calendars of the last run until the server answers
const CACHE_FILE: &str = "calendar.json";

#[derive(Debug)]
pub struct Manager {
    pub client: Client,
    map: CalendarMap,
}

impl Manager {
    pub fn new(ical: config::Ical) -> Self {
        Self {
            client: Client::new(ical.url.clone(), Credentials::from(ical)),
            map: load_cache(),
        }
    }

    // Returns true if the given map is different from the current map
    pub fn set_map(&mut self, map: CalendarMap) -> bool {
        let old_map = std::mem::replace(&mut self.map, map);
        let changed = old_map != self.map;

        if changed {
            save_cache(&self.map);
        }

        changed
    }

    // Returns true if the fetched map is different from the current map
    // pub fn update(&mut self) -> bool {
    //     match self.client.get_map() {
    //         Ok(map) => self.set_map(map),
    //         Err(e) => {
    //             log::error!("Error fetching calendar map: {:?}", e);
    //             false
    //         }
    //     }
    // }

    pub fn calendars<'a>(
        &'a self,
        filter: Option<&'a config::UuidFilter>,
    ) -> impl Iterator<Item = (&'a Uuid, &'a (bool, Calendar))> {
        self.map
            .calendars()
            .iter()
            .filter(move |(uid, _)| filter.is_none_or(|filter| filter.is_included(uid)))
    }

    pub fn toggle_calendar(&mut self, uid: Uuid) {
        self.map.toggle_calendar(uid);
    }

    pub fn events_between<'a>(
        &'a self,
        start: NaiveDate,
        end: NaiveDate,
        filter: Option<&'a UuidFilter>,
    ) -> impl Iterator<Item = (&'a Calendar, &'a NaiveDateTime, &'a Event)> {
        self.map
            .events_between(start, end)
            .filter(move |(calendar, _, _)| {
                filter.is_none_or(|filter| filter.is_included(&calendar.uid))
            })
    }

    pub fn calendars_between<'a>(
        &'a self,
        start: NaiveDate,
        end: NaiveDate,
        filter: Option<&'a UuidFilter>,
    ) -> impl Iterator<Item = (&'a NaiveDateTime, &'a Calendar)> {
        self.map
            .calendars_between(start, end)
            .filter(move |(_, calendar)| {
                filter.is_none_or(|filter| filter.is_included(&calendar.uid))
            })
    }

    pub fn calendar_name(&self, uid: &Uuid) -> Option<String> {
        self.map.calendars().get(uid).map(|(_, calendar)| calendar.name.to_string())
    }
}

fn load_cache() -> CalendarMap {
    let Some(map) = cache::path(CACHE_FILE)
        .and_then(|path| std::fs::read(path).ok())
        .and_then(|contents| {
            serde_json::from_slice::<CalendarMap>(&contents)
                .inspect_err(|err| log::warn!("Calendar: ignoring broken cache: {err}"))
                .ok()
        })
    else {
        return CalendarMap::default();
    };

    log::info!(
        "Calendar: loaded {} calendars, {} events from cache",
        map.len_calendars(),
        map.len_events()
    );

    map
}

fn save_cache(map: &CalendarMap) {
    let Some(path) = cache::path(CACHE_FILE) else {
        return;
    };

    let result = serde_json::to_vec(map)
        .map_err(std::io::Error::other)
        .and_then(|contents| cache::write(&path, &contents));

    if let Err(err) = result {
        log::warn!("Calendar: failed to write cache {}: {err}", path.display());
    }
}
