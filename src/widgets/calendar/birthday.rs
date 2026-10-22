//! The next birthday, as a banner above the day list and as a line on the screensaver.
//!
//! Which calendars hold birthdays is configured, they are not guessed from their names.

use chrono::{Days, NaiveDate};

use crate::calendar::Manager;
use crate::config::{Config, UuidFilter};
use crate::prelude::*;

const ICON_CAKE: char = '\u{f1fd}'; // cake-candles

/// Birthdays further away than this are not worth a banner
const WITHIN_DAYS: u64 = 7;

/// What the banner says about the next birthday, or all of them when they share a day.
struct Birthday {
    /// Everyone whose birthday falls on `date`, in alphabetical order
    names: Vec<String>,
    date: NaiveDate,
    /// Days from today, 0 while it is today
    in_days: i64,
}

impl Birthday {
    /// All the names, joined. Several people share one banner rather than hiding each other.
    fn names(&self) -> String {
        self.names.join(", ")
    }
}

/// The next birthday today or within the next few days, with everyone who shares that day.
///
/// Without a configured filter there is nothing to show: an empty filter would mean
/// "every calendar", which would put any appointment under a cake.
fn next(manager: &Manager, filter: Option<&UuidFilter>, today: NaiveDate) -> Option<Birthday> {
    let filter = filter?;
    let until = today + Days::new(WITHIN_DAYS);

    let birthdays = manager
        .events_between(today, until, Some(filter))
        .map(|(_, occurrence, event)| (occurrence.date(), event.summary.clone()))
        .collect();

    earliest(birthdays, today)
}

/// Everyone from the earliest day of the list, which puts today's birthdays before the
/// ones still coming.
fn earliest(mut birthdays: Vec<(NaiveDate, String)>, today: NaiveDate) -> Option<Birthday> {
    // Sorting by day and then by name also groups the people who share a day
    birthdays.sort_unstable();
    birthdays.dedup();

    let date = birthdays.first()?.0;

    Some(Birthday {
        names: birthdays
            .into_iter()
            .take_while(|(day, _)| *day == date)
            .map(|(_, summary)| summary)
            .collect(),
        date,
        in_days: (date - today).num_days(),
    })
}

fn countdown(in_days: i64) -> String {
    match in_days {
        ..=0 => "heute".to_string(),
        1 => "morgen".to_string(),
        days => format!("in {days} Tagen"),
    }
}

fn format_date(date: NaiveDate) -> String {
    date.format_localized("%A, %-d. %B", chrono::Locale::de_DE)
        .to_string()
}

/// The banner above the day list, hidden while no birthday is near.
pub struct BirthdayWidget {
    filter: Option<UuidFilter>,
    wrapper: gtk::Box,
    name: gtk::Label,
    date: gtk::Label,
    countdown: gtk::Label,
}

impl BirthdayWidget {
    pub fn new(config: &Config) -> Self {
        let icon = gtk::Label::builder()
            .label(ICON_CAKE.to_string())
            .valign(gtk::Align::Center)
            .css_classes(["calendar-birthday__icon", "icon"])
            .build();

        let name = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(["calendar-birthday__name"])
            .build();

        let date = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .css_classes(["calendar-birthday__date"])
            .build();

        let labels = gtk::Box::new(gtk::Orientation::Vertical, 2);
        labels.set_hexpand(true);
        labels.set_valign(gtk::Align::Center);
        labels.append(&name);
        labels.append(&date);

        let countdown = gtk::Label::builder()
            .halign(gtk::Align::End)
            .valign(gtk::Align::Center)
            .css_classes(["calendar-birthday__countdown"])
            .build();

        let wrapper = gtk::Box::new(gtk::Orientation::Horizontal, 18);
        wrapper.add_css_class("calendar-birthday");
        wrapper.set_visible(false);
        wrapper.append(&icon);
        wrapper.append(&labels);
        wrapper.append(&countdown);

        Self {
            filter: config.calendar.birthday.clone(),
            wrapper,
            name,
            date,
            countdown,
        }
    }

    pub const fn widget(&self) -> &gtk::Box {
        &self.wrapper
    }

    /// Always about today, no matter which day is selected.
    pub fn update_calendar(&mut self, manager: &Manager, dates: &super::Dates) {
        let birthday = next(manager, self.filter.as_ref(), dates.today());

        if let Some(birthday) = &birthday {
            self.name.set_label(&birthday.names());
            self.date.set_label(&format_date(birthday.date));
            self.countdown.set_label(&countdown(birthday.in_days));
        }

        self.wrapper.set_visible(birthday.is_some());
    }
}

/// The same birthday as one quiet line on the screensaver.
pub struct BirthdayLine {
    filter: Option<UuidFilter>,
    wrapper: gtk::Box,
    name: gtk::Label,
    countdown: gtk::Label,
}

impl BirthdayLine {
    pub fn new(config: &Config) -> Self {
        let icon = gtk::Label::builder()
            .label(ICON_CAKE.to_string())
            .css_classes(["screensaver__birthday__icon", "icon"])
            .build();

        let name = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(["screensaver__birthday"])
            .build();

        let countdown = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .css_classes(["screensaver__birthday__countdown"])
            .build();

        let wrapper = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        wrapper.add_css_class("screensaver__birthday__row");
        wrapper.set_halign(gtk::Align::Start);
        wrapper.set_visible(false);
        wrapper.append(&icon);
        wrapper.append(&name);
        wrapper.append(&countdown);

        Self {
            filter: config.calendar.birthday.clone(),
            wrapper,
            name,
            countdown,
        }
    }

    pub const fn widget(&self) -> &gtk::Box {
        &self.wrapper
    }

    pub fn update_calendar(&mut self, manager: &Manager, dates: &super::Dates) {
        let birthday = next(manager, self.filter.as_ref(), dates.today());

        if let Some(birthday) = &birthday {
            self.name.set_label(&birthday.names());
            self.countdown.set_label(&countdown(birthday.in_days));
        }

        self.wrapper.set_visible(birthday.is_some());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
    }

    fn entries(items: &[(u32, &str)]) -> Vec<(NaiveDate, String)> {
        items
            .iter()
            .map(|(day, name)| (date(*day), (*name).to_string()))
            .collect()
    }

    #[test]
    fn finds_nothing_without_birthdays() {
        assert!(earliest(Vec::new(), date(22)).is_none());
    }

    #[test]
    fn keeps_everyone_who_shares_the_earliest_day() {
        let found = earliest(entries(&[(24, "Opa"), (24, "Oma"), (26, "Jan")]), date(22)).unwrap();

        assert_eq!(found.names, ["Oma", "Opa"]);
        assert_eq!(found.names(), "Oma, Opa");
        assert_eq!(found.date, date(24));
        assert_eq!(found.in_days, 2);
    }

    #[test]
    fn prefers_today_over_later_days() {
        let found = earliest(entries(&[(26, "Jan"), (22, "Oma")]), date(22)).unwrap();

        assert_eq!(found.names, ["Oma"]);
        assert_eq!(found.in_days, 0);
        assert_eq!(countdown(found.in_days), "heute");
    }

    #[test]
    fn collapses_the_same_birthday_twice() {
        let found = earliest(entries(&[(23, "Oma"), (23, "Oma")]), date(22)).unwrap();

        assert_eq!(found.names, ["Oma"]);
        assert_eq!(countdown(found.in_days), "morgen");
    }
}
