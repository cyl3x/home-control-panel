use chrono::{Days, TimeDelta};

use crate::calendar::{Calendar, Manager};
use crate::config::{Config, UpcomingFilter, UuidFilter};
use crate::widgets::calendar::Dates;
use crate::{calendar, prelude::*};

pub struct UpcomingWidget {
    config: Option<UpcomingFilter>,
    filter: Option<UuidFilter>,
    wrapper: gtk::Box,
    grid: Option<gtk::Grid>,
}

impl UpcomingWidget {
    pub fn new(config: &Config) -> Self {
        let wrapper = gtk::Box::new(gtk::Orientation::Vertical, 0);
        wrapper.set_halign(gtk::Align::Center);
        wrapper.set_valign(gtk::Align::Center);

        let filter = config.calendar.upcomming.as_ref().map(|config| UuidFilter {
            exclude: config.exclude.clone(),
            include: config.include.clone(),
        });

        Self {
            filter,
            config: config.calendar.upcomming.clone(),
            wrapper,
            grid: None,
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
        if let Some(grid) = self.grid.take() {
            self.wrapper.remove(&grid);
        }

        let now = dates.now.naive_utc().date();

        let grid = gtk::Grid::new();
        grid.add_css_class("calendar-upcoming");
        grid.set_row_homogeneous(true);
        grid.set_column_homogeneous(false);
        grid.set_row_spacing(32);
        grid.set_column_spacing(24);

        let mut grid_row = 0;

        if let Some(events) = self.map_events(manager, now, now, now) {
            let label = Self::create_name("Heute");
            grid.attach(&label, 0, grid_row, 1, 1);
            grid.attach(&events, 1, grid_row, 1, 1);
            grid_row += 1;
        }

        if let Some(events) = self.map_events(manager, now, now + Days::new(1), now + Days::new(1))
        {
            let label = Self::create_name("Morgen");
            grid.attach(&label, 0, grid_row, 1, 1);
            grid.attach(&events, 1, grid_row, 1, 1);
            grid_row += 1;
        }

        if now.weekday() == Weekday::Fri
            && let Some(events) =
                self.map_events(manager, now, now + Days::new(2), now + Days::new(2))
        {
            let label = Self::create_name("Sonntag");
            grid.attach(&label, 0, grid_row, 1, 1);
            grid.attach(&events, 1, grid_row, 1, 1);
        } else if !matches!(now.weekday(), Weekday::Sat | Weekday::Sun) {
            let num = u64::from(now.weekday().num_days_from_monday());
            if let Some(events) = self.map_events(
                manager,
                now,
                now + Days::new(5 - num),
                now + Days::new(6 - num),
            ) {
                let label = Self::create_name("Nächstes Wochenende");
                grid.attach(&label, 0, grid_row, 1, 1);
                grid.attach(&events, 1, grid_row, 1, 1);
            }
        }

        self.wrapper.append(&grid);

        self.grid = Some(grid);
    }

    fn create_name(name: &str) -> gtk::Label {
        let label = gtk::Label::new(Some(name));
        label.set_valign(gtk::Align::Start);
        label.set_halign(gtk::Align::End);

        label
    }

    fn map_events<'a>(
        &'a self,
        manager: &'a Manager,
        now: NaiveDate,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Option<gtk::Box> {
        let events = manager
            .events_between(from, to, self.filter.as_ref())
            .map(|item| self.create_event(item, now))
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
        (calendar, _, event): (&'a Calendar, &NaiveDateTime, &'a calendar::Event),
        now: NaiveDate,
    ) -> gtk::Box {
        let indicator = gtk::Box::new(gtk::Orientation::Vertical, 0);
        indicator.add_css_class("calendar-upcoming__item__indicator");
        indicator.set_halign(gtk::Align::Start);
        indicator.set_valign(gtk::Align::Center);
        indicator.set_height_request(12);
        indicator.set_width_request(12);
        indicator.inline_css(&format!("background-color: {}", calendar.css_color()));

        let text = if self.should_skip(&calendar.uid) {
            event.summary.clone()
        } else {
            self.oneliner(event, now)
        };

        let label = gtk::Label::new(Some(&text));
        label.add_css_class("calendar-upcoming__item__label");

        let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        row.add_css_class("calendar-upcoming__item");
        row.set_halign(gtk::Align::Start);
        row.append(&indicator);
        row.append(&label);

        row
    }

    fn oneliner(&self, event: &calendar::Event, now: NaiveDate) -> String {
        let start = event.start_tz();
        let end = event.end_tz();

        let delta = event.end - event.start;
        let is_delta_whole_days =
            event.start.and_utc().hour() == 0 && delta.num_days() * 86400 == delta.num_seconds();

        let mut info = String::new();

        if event.start_date() != now {
            info.push_str(
                &start
                    .format_localized("%a", chrono::Locale::de_DE)
                    .to_string(),
            );
        }

        if !is_delta_whole_days {
            info.push(' ');
            info.push_str(&start.format("%H:%M").to_string());
        }

        info.push_str(" -");

        if !(is_delta_whole_days && delta == TimeDelta::days(1)
            || delta.is_zero()
            || event.end_date() == event.start_date())
        {
            info.push(' ');
            info.push_str(
                &end.format_localized("%a", chrono::Locale::de_DE)
                    .to_string(),
            );
        }

        if !is_delta_whole_days && !delta.is_zero() {
            info.push(' ');
            info.push_str(&end.format("%H:%M").to_string());
        }

        info = info.trim_end_matches(" -").trim().to_string();

        if info.is_empty() {
            event.summary.clone()
        } else {
            format!("{} ({info})", event.summary)
        }
    }
}
