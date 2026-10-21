use std::collections::BTreeSet;

use chrono::{DateTime, Datelike, Days, Local, NaiveDate, NaiveDateTime, Timelike};
use rrule::Unvalidated;
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Event {
    pub etag: String,
    pub uid: Uuid,
    pub summary: String,
    pub description: Option<String>,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub url: Url,
    #[serde(with = "rrule_text")]
    pub rrule: Option<rrule::RRule<Unvalidated>>,
}

impl Event {
    pub fn description(&self) -> &str {
        self.description.as_deref().unwrap_or_default()
    }

    pub fn tooltip(&self) -> String {
        if self.start == self.end {
            return format!("{}\n{}", self.summary, format_date(&self.start_tz()));
        }

        format!(
            "{}\n{} - {}",
            self.summary,
            format_date(&self.start_tz()),
            format_date(&self.end_tz()),
        )
    }

    pub const fn start_date(&self) -> NaiveDate {
        self.start.date()
    }

    pub fn start_tz(&self) -> DateTime<Local> {
        self.start.and_utc().with_timezone(&Local)
    }

    pub fn end_date(&self) -> NaiveDate {
        if self.end.hour() == 0 && self.end.minute() == 0 {
            self.end.date() - Days::new(1)
        } else {
            self.end.date()
        }
    }

    pub fn end_tz(&self) -> DateTime<Local> {
        self.end.and_utc().with_timezone(&Local)
    }

    pub fn start_end_dates(&self) -> (NaiveDate, NaiveDate) {
        (self.start_date(), self.end_date())
    }

    pub fn all_date_times(&self) -> BTreeSet<NaiveDateTime> {
        self.rrule.as_ref().map_or_else(
            || dates_between(self.start, self.end),
            |rrule| {
                let limit = chrono::Utc::now().date_naive() + chrono::Duration::days(365 + 2);

                let interval = self.end - self.start;
                let start = self.start.and_utc().with_timezone(&rrule::Tz::LOCAL);
                let set = rrule.clone().build(start).unwrap();

                set.into_iter()
                    .map(|date| date.naive_utc())
                    .take_while(move |start| start.date() < limit)
                    .flat_map(move |start| dates_between(start, start + interval))
                    .collect()
            },
        )
    }
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.uid == other.uid && self.etag == other.etag
    }
}

fn dates_between(start: NaiveDateTime, end: NaiveDateTime) -> BTreeSet<NaiveDateTime> {
    if start == end {
        return [start].into();
    }

    start
        .date()
        .iter_days()
        .map(|date| date.and_time(start.time()))
        .take_while(|start| start < &end)
        .collect()
}

/// Persist recurrence rules in their iCalendar text form, which the rrule crate prints and parses.
mod rrule_text {
    use rrule::{RRule, Unvalidated};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(
        rrule: &Option<RRule<Unvalidated>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        rrule
            .as_ref()
            .map(ToString::to_string)
            .serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<RRule<Unvalidated>>, D::Error> {
        Option::<String>::deserialize(deserializer)?
            .map(|text| text.parse().map_err(serde::de::Error::custom))
            .transpose()
    }
}

fn format_date(date: &DateTime<Local>) -> String {
    let now = Local::now();

    let formatted = if date.year() == now.year() {
        date.format_localized("%m. %b %H:%M", chrono::Locale::de_DE)
    } else {
        date.format_localized("%m. %b %Y %H:%M", chrono::Locale::de_DE)
    };

    formatted.to_string()
}
