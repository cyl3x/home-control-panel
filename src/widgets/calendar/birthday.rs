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

/// What the banner says about the next birthday.
struct Birthday {
    summary: String,
    date: NaiveDate,
    /// Days from today, 0 while it is today
    in_days: i64,
}

/// The next birthday today or within the next few days, the earliest one first.
///
/// Without a configured filter there is nothing to show: an empty filter would mean
/// "every calendar", which would put any appointment under a cake.
fn next(manager: &Manager, filter: Option<&UuidFilter>, today: NaiveDate) -> Option<Birthday> {
    let filter = filter?;
    let until = today + Days::new(WITHIN_DAYS);

    manager
        .events_between(today, until, Some(filter))
        .map(|(_, occurrence, event)| (occurrence.date(), event))
        .min_by_key(|(date, _)| *date)
        .map(|(date, event)| Birthday {
            summary: event.summary.clone(),
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
            self.name.set_label(&birthday.summary);
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
            self.name.set_label(&birthday.summary);
            self.countdown.set_label(&countdown(birthday.in_days));
        }

        self.wrapper.set_visible(birthday.is_some());
    }
}
