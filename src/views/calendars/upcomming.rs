use chrono::{Datelike, Days, NaiveDate, NaiveDateTime, TimeDelta, Timelike, Weekday};
use iced::widget::{column, container, row, text};
use iced::{Alignment, Color, Padding};
use iced_font_awesome::fa_icon_solid;
use uuid::Uuid;

use crate::calendar::{self, Calendar, Manager};
use crate::config;

use super::{Dates, Message};

pub struct Upcomming {
    config: Option<config::UpcommingFilter>,
    filter: Option<config::UuidFilter>,
}

impl Upcomming {
    pub fn new(config: Option<config::UpcommingFilter>) -> Self {
        let filter = config.as_ref().map(|config| config::UuidFilter {
            exclude: config.exclude.clone(),
            include: config.include.clone(),
        });

        Self { filter, config }
    }

    pub fn view<'a>(&'a self, manager: &'a Manager, dates: &'a Dates) -> iced::Element<'a, Message> {
        let now = dates.now.date();

        let mut dates = column![].spacing(8).align_x(Alignment::End);
        let mut events = column![].spacing(8);

        let today = self.map_events(manager, now, now, now);
        dates = dates.push_maybe(self.view_name("Heute", today.len()));
        events = events.push_maybe(self.view_events(today));

        let tomorrow = self.map_events(manager, now, now + Days::new(1), now + Days::new(1));
        dates = dates.push_maybe(self.view_name("Morgen", tomorrow.len()));
        events = events.push_maybe(self.view_events(tomorrow));

        if now.weekday() == Weekday::Fri {
            let sunday = self.map_events(manager, now, now + Days::new(2), now + Days::new(2));
            dates = dates.push_maybe(self.view_name("Sontag", sunday.len()));
            events = events.push_maybe(self.view_events(sunday));
        } else if !matches!(now.weekday(), Weekday::Sat | Weekday::Sun) {
            let num = now.weekday().num_days_from_monday() as u64;
            let weekend = self.map_events(manager, now, now + Days::new(5 - num), now + Days::new(6 - num));
            dates = dates.push_maybe(self.view_name("Nächstes Wochenende", weekend.len()));
            events = events.push_maybe(self.view_events(weekend));
        }

        row![dates, events].spacing(16).into()
    }

    fn view_name<'a>(&'a self, name: &'a str, len: usize) -> Option<iced::Element<'a, Message>> {
        if len == 0 {
            return None;
        }

        let name = text(name)
            .color(Color::from_rgb8(192, 192, 192))
            .size(24);

        Some(column([name.into()])
            .extend((1..len).map(|_| text("").into()))
            .spacing(8).into())
    }

    fn view_events<'a>(&'a self, events: Vec<iced::Element<'a, Message>>) -> Option<iced::Element<'a, Message>> {
        if events.is_empty() {
            return None;
        }

        Some(column(events).spacing(8).into())
    }

    fn view_event<'a>(
        &'a self,
        (calendar, _, event): (&'a Calendar, &NaiveDateTime, &'a calendar::Event),
        now: NaiveDate,
    ) -> iced::Element<'a, Message> {
        let icon = fa_icon_solid("circle").color(calendar.color).size(12.0);

        row![
            container(icon).padding(Padding::ZERO.top(9.0)),
            text(if self.should_skip(&calendar.uid) {
                event.summary.clone()
            } else {
                self.oneliner(event, now)
            }).size(24).color(Color::from_rgb8(192, 192, 192)),
        ]
        .spacing(8)
        .into()
    }

    fn map_events<'a>(
        &'a self,
        manager: &'a Manager,
        now: NaiveDate,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Vec<iced::Element<'a, Message>> {
        manager
            .events_between(from, to, self.filter.as_ref())
            .map(|item| self.view_event(item, now))
            .collect::<Vec<_>>()
    }

    // @TODO: start and end times for rrules are out of place
    fn oneliner(&self, event: &calendar::Event, now: NaiveDate) -> String {
        let start = event.start_tz();
        let end = event.end_tz();

        let delta = event.end - event.start;
        let is_delta_whole_days = event.start.and_utc().hour() == 0 && delta.num_days() * 86400 == delta.num_seconds();

        let mut info = String::new();

        if event.start_date() != now {
            info.push_str(&start.format_localized("%a", chrono::Locale::de_DE).to_string());
        };

        if !is_delta_whole_days {
            info.push(' ');
            info.push_str(&start.format("%H:%M").to_string())
        };

        info.push_str(" -");

        if !(is_delta_whole_days && delta == TimeDelta::days(1) || delta.is_zero() || event.end_date() == event.start_date()) {
            info.push(' ');
            info.push_str(&end.format_localized("%a", chrono::Locale::de_DE).to_string());
        };

        if !is_delta_whole_days && !delta.is_zero() {
            info.push(' ');
            info.push_str(&end.format("%H:%M").to_string())
        };

        info = info.trim_end_matches(" -").trim().to_string();

        if info.is_empty() {
            event.summary.clone()
        } else {
            format!("{} ({info})", event.summary)
        }
    }

    fn should_skip(&self, uid: &Uuid) -> bool {
        self.config.as_ref().is_some_and(|c| c.skip_oneliner.contains(uid))
    }
}
