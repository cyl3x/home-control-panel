//! The agenda of the screensaver, grouped by day with the start times in their own column.

use chrono::{Days, Timelike};

use crate::calendar::{Calendar, Manager};
use crate::config::{Config, UpcomingFilter, UuidFilter};
use crate::widgets::calendar::Dates;
use crate::{calendar, prelude::*};

pub struct UpcomingWidget {
    config: Option<UpcomingFilter>,
    filter: Option<UuidFilter>,
    wrapper: gtk::Box,
    groups: Option<gtk::Box>,
}

impl UpcomingWidget {
    pub fn new(config: &Config) -> Self {
        let wrapper = gtk::Box::new(gtk::Orientation::Vertical, 0);
        wrapper.set_halign(gtk::Align::Start);
        wrapper.set_valign(gtk::Align::Center);

        let filter = config.calendar.upcomming.as_ref().map(|config| UuidFilter {
            exclude: config.exclude.clone(),
            include: config.include.clone(),
        });

        Self {
            filter,
            config: config.calendar.upcomming.clone(),
            wrapper,
            groups: None,
        }
    }

    pub const fn widget(&self) -> &gtk::Box {
        &self.wrapper
    }

    fn should_skip(&self, uid: &uuid::Uuid) -> bool {
        self.config
            .as_ref()
            .is_some_and(|c| c.skip_oneliner.contains(uid))
    }

    pub fn update_calendar(&mut self, manager: &Manager, dates: &Dates) {
        if let Some(groups) = self.groups.take() {
            self.wrapper.remove(&groups);
        }

        let now = dates.today();

        let groups = gtk::Box::new(gtk::Orientation::Vertical, 24);
        groups.add_css_class("calendar-upcoming");

        if let Some(events) = self.map_events(manager, now, now, false) {
            groups.append(&Self::group("HEUTE", &events));
        }

        let tomorrow = now + Days::new(1);
        if let Some(events) = self.map_events(manager, tomorrow, tomorrow, false) {
            groups.append(&Self::group("MORGEN", &events));
        }

        if now.weekday() == Weekday::Fri {
            let sunday = now + Days::new(2);
            if let Some(events) = self.map_events(manager, sunday, sunday, false) {
                groups.append(&Self::group("SONNTAG", &events));
            }
        } else if !matches!(now.weekday(), Weekday::Sat | Weekday::Sun) {
            let num = u64::from(now.weekday().num_days_from_monday());
            if let Some(events) =
                self.map_events(manager, now + Days::new(5 - num), now + Days::new(6 - num), true)
            {
                groups.append(&Self::group("NÄCHSTES WOCHENENDE", &events));
            }
        }

        self.wrapper.append(&groups);

        self.groups = Some(groups);
    }

    /// A heading with its events below it.
    fn group(name: &str, events: &gtk::Box) -> gtk::Box {
        let label = gtk::Label::new(Some(name));
        label.set_halign(gtk::Align::Start);
        label.add_css_class("calendar-upcoming__group");

        let section = gtk::Box::new(gtk::Orientation::Vertical, 6);
        section.set_halign(gtk::Align::Start);
        section.append(&label);
        section.append(events);

        section
    }

    /// `spans_days` tells whole-day events to name their weekday, which only helps
    /// for groups that cover more than one day.
    fn map_events(
        &self,
        manager: &Manager,
        from: NaiveDate,
        to: NaiveDate,
        spans_days: bool,
    ) -> Option<gtk::Box> {
        let events = manager
            .events_between(from, to, self.filter.as_ref())
            .map(|item| self.create_event(item, spans_days))
            .collect::<Vec<_>>();

        if events.is_empty() {
            return None;
        }

        let wrapper = gtk::Box::new(gtk::Orientation::Vertical, 0);
        for event in events {
            wrapper.append(&event);
        }

        Some(wrapper)
    }

    fn create_event<'a>(
        &'a self,
        (calendar, occurrence, event): (&'a Calendar, &NaiveDateTime, &'a calendar::Event),
        spans_days: bool,
    ) -> gtk::Box {
        let indicator = gtk::Box::new(gtk::Orientation::Vertical, 0);
        indicator.add_css_class("calendar-upcoming__item__indicator");
        indicator.set_halign(gtk::Align::Start);
        indicator.set_valign(gtk::Align::Center);
        indicator.set_height_request(10);
        indicator.set_width_request(10);
        indicator.inline_css(&format!("background-color: {}", calendar.css_color()));

        let when = if self.should_skip(&calendar.uid) {
            String::new()
        } else {
            Self::when(event, *occurrence, spans_days)
        };

        let time = gtk::Label::new(Some(&when));
        time.set_halign(gtk::Align::Start);
        time.add_css_class("calendar-upcoming__item__time");

        let label = gtk::Label::new(Some(&event.summary));
        label.set_halign(gtk::Align::Start);
        label.add_css_class("calendar-upcoming__item__label");

        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        row.add_css_class("calendar-upcoming__item");
        row.set_halign(gtk::Align::Start);
        row.append(&indicator);
        row.append(&time);
        row.append(&label);

        row
    }

    /// The start time, or the weekday for events that last whole days.
    fn when(event: &calendar::Event, occurrence: NaiveDateTime, spans_days: bool) -> String {
        let duration = event.end - event.start;
        let whole_days = occurrence.and_utc().hour() == 0
            && duration.num_days() * 86400 == duration.num_seconds();

        let start = occurrence.and_utc().with_timezone(&Local);

        if !whole_days {
            return start.format("%H:%M").to_string();
        }

        if spans_days {
            return start
                .format_localized("%a", chrono::Locale::de_DE)
                .to_string();
        }

        String::new()
    }
}
