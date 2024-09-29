use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use uuid::Uuid;

use super::{calendar::Calendar, event::Event};

#[derive(Debug, PartialEq, Default)]
pub struct CalendarMap {
    // Map of calendars by their uid
    calendars: BTreeMap<Uuid, (bool, Calendar)>,
    // Map of events and their calendar_uid by their uid
    events: BTreeMap<Uuid, (Uuid, Event)>,
    // Map of dates to events and their start time
    event_map: BTreeMap<NaiveDateTime, BTreeSet<Uuid>>,
    // Map of dates to events and their start time
    calendar_map: BTreeMap<NaiveDateTime, BTreeSet<Uuid>>,
}

impl CalendarMap {
    pub const fn calendars(&self) -> &BTreeMap<Uuid, (bool, Calendar)> {
        &self.calendars
    }

    pub fn toggle_calendar(&mut self, uid: Uuid) {
        if let Some((enabled, _)) = self.calendars.get_mut(&uid) {
            *enabled = !*enabled;
        }
    }

    pub fn clear(&mut self) {
        self.calendars.clear();
        self.events.clear();
        self.event_map.clear();
    }

    pub fn add_calendar(&mut self, calendar: Calendar) {
        self.calendars.insert(calendar.uid, (true, calendar));
    }

    pub fn add_event(&mut self, calendar_uid: Uuid, event: Event) {
        for date_time in event.all_date_times() {
            self.event_map
                .entry(date_time)
                .or_default()
                .insert(event.uid);

            self.calendar_map
                .entry(date_time)
                .or_default()
                .insert(calendar_uid);
        }

        self.events.insert(event.uid, (calendar_uid, event));
    }

    pub fn events_between(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> impl Iterator<Item = (&Calendar, &NaiveDateTime, &Event)> {
        self.event_map
            .range(range(start, end))
            .flat_map(move |(date_time, id_set)| {
                id_set.iter().filter_map(move |uid| {
                    let (calendar_uid, event) = self.events.get(uid).unwrap();
                    let (enabled, calendar) = self.calendars.get(calendar_uid).unwrap();

                    match enabled {
                        true => Some((calendar, date_time, event)),
                        _ => None,
                    }
                })
            })
    }

    pub fn calendars_between(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> impl Iterator<Item = (&NaiveDateTime, &Calendar)> {
        self.calendar_map
            .range(range(start, end))
            .flat_map(move |(date_time, id_set)| {
                id_set.iter().filter_map(move |uid| {
                    let (enabled, calendar) = self.calendars.get(uid).unwrap();

                    match enabled {
                        true => Some((date_time, calendar)),
                        _ => None,
                    }
                })
            })
    }

    pub fn len_events(&self) -> usize {
        self.events.len()
    }

    pub fn len_calendars(&self) -> usize {
        self.calendars.len()
    }
}

fn range(start: NaiveDate, end: NaiveDate) -> Range<NaiveDateTime> {
    start.and_time(NaiveTime::MIN)..(end.and_time(NaiveTime::MIN) + chrono::Duration::days(1))
}
