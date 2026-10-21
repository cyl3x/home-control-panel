use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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

/// Only calendars and events are persisted, the maps by date are built from them when loading.
#[derive(Serialize)]
struct PersistedRef<'a> {
    calendars: Vec<&'a Calendar>,
    events: Vec<&'a (Uuid, Event)>,
}

#[derive(Deserialize)]
struct Persisted {
    calendars: Vec<Calendar>,
    events: Vec<(Uuid, Event)>,
}

impl Serialize for CalendarMap {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        PersistedRef {
            calendars: self
                .calendars
                .values()
                .map(|(_, calendar)| calendar)
                .collect(),
            events: self.events.values().collect(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CalendarMap {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let persisted = Persisted::deserialize(deserializer)?;
        let mut map = Self::default();

        for calendar in persisted.calendars {
            map.add_calendar(calendar);
        }

        for (calendar_uid, event) in persisted.events {
            map.add_event(calendar_uid, event);
        }

        Ok(map)
    }
}

fn range(start: NaiveDate, end: NaiveDate) -> Range<NaiveDateTime> {
    start.and_time(NaiveTime::MIN)..(end.and_time(NaiveTime::MIN) + chrono::Duration::days(1))
}

#[cfg(test)]
mod tests {
    use palette::rgb::Rgb;

    use super::*;

    fn event(uid: u128, rrule: Option<&str>) -> Event {
        Event {
            etag: format!("etag-{uid}"),
            uid: Uuid::from_u128(uid),
            summary: "Event".to_string(),
            description: Some("Description".to_string()),
            start: NaiveDate::from_ymd_opt(2026, 3, 2)
                .unwrap()
                .and_hms_opt(17, 30, 0)
                .unwrap(),
            end: NaiveDate::from_ymd_opt(2026, 3, 2)
                .unwrap()
                .and_hms_opt(19, 0, 0)
                .unwrap(),
            url: "https://example.com/calendar/event.ics".parse().unwrap(),
            rrule: rrule.map(|rrule| rrule.parse().unwrap()),
        }
    }

    #[test]
    fn survives_persisting() {
        let calendar = Calendar {
            uid: Uuid::from_u128(1),
            url_str: "https://example.com/calendar/".to_string(),
            name: "Calendar".to_string(),
            color: Rgb::new(222, 184, 135),
        };

        let mut map = CalendarMap::default();
        map.add_calendar(calendar.clone());
        map.add_event(calendar.uid, event(10, None));
        map.add_event(
            calendar.uid,
            event(11, Some("FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE")),
        );
        map.add_event(
            calendar.uid,
            event(
                12,
                Some("FREQ=MONTHLY;UNTIL=20271231T230000Z;BYMONTHDAY=15"),
            ),
        );
        map.add_event(
            calendar.uid,
            event(13, Some("FREQ=YEARLY;COUNT=5;BYMONTH=3")),
        );

        let persisted: CalendarMap =
            serde_json::from_slice(&serde_json::to_vec(&map).unwrap()).unwrap();

        // Includes the dates of the recurrences, which are derived from the rules
        assert_eq!(persisted, map);
        assert_eq!(persisted.calendars()[&calendar.uid].1.color, calendar.color);

        for (uid, (_, event)) in &map.events {
            assert_eq!(
                persisted.events[uid]
                    .1
                    .rrule
                    .as_ref()
                    .map(ToString::to_string),
                event.rrule.as_ref().map(ToString::to_string)
            );
        }
    }
}
