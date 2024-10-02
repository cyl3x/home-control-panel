use std::collections::BTreeSet;

use chrono::{DateTime, Datelike, Days, NaiveDate, NaiveDateTime, Timelike};
use rrule::Unvalidated;
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Event {
    pub etag: String,
    pub uid: Uuid,
    pub summary: String,
    pub description: Option<String>,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub url: Url,
    pub rrule: Option<rrule::RRule<Unvalidated>>,
}

impl Event {
    pub fn description(&self) -> &str {
        self.description.as_deref().unwrap_or_default()
    }

    pub fn tooltip(&self) -> String {
        if self.start == self.end {
            return format!("{}\n{}", self.summary, format_date(&self.start_tz()),);
        }

        format!(
            "{}\n{} - {}",
            self.summary,
            format_date(&self.start_tz()),
            format_date(&self.end_tz()),
        )
    }

    pub fn rrule_start(&self) -> DateTime<rrule::Tz> {
        self.start.and_utc().with_timezone(&rrule::Tz::UTC)
    }

    pub const fn start_date(&self) -> NaiveDate {
        self.start.date()
    }

    pub fn start_tz(&self) -> DateTime<chrono_tz::Tz> {
        self.start
            .and_utc()
            .with_timezone(&chrono_tz::Europe::Berlin)
    }

    pub fn end_date(&self) -> NaiveDate {
        if self.end.hour() == 0 && self.end.minute() == 0 {
            self.end.date() - Days::new(1)
        } else {
            self.end.date()
        }
    }

    pub fn end_tz(&self) -> DateTime<chrono_tz::Tz> {
        self.end.and_utc().with_timezone(&chrono_tz::Europe::Berlin)
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
                let start = self.rrule_start();
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

fn format_date(date: &DateTime<chrono_tz::Tz>) -> String {
    let now = chrono::Utc::now().with_timezone(&date.timezone());

    let formatted = if date.year() == now.year() {
        date.format_localized("%m. %b %H:%M", chrono::Locale::de_DE)
    } else {
        date.format_localized("%m. %b %Y %H:%M", chrono::Locale::de_DE)
    };

    formatted.to_string()
}
