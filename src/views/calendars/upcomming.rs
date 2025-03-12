use chrono::{Datelike, Days, NaiveDate, NaiveDateTime, TimeDelta, Timelike, Weekday};
use iced::widget::{column, container, row, text};
use iced::{Alignment, Length, Padding};
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
        let saturday = self.current_saturday(now);

        column![]
            .push_maybe(self.view_row(manager, now, "Heute", now, now))
            .push_maybe(self.view_row(manager, now, "Morgen", now + Days::new(1), now + Days::new(1)))
            .push_maybe(self.view_row(
                manager,
                dates.now.date(),
                if dates.now.date() >= saturday { "Dieses Wochenende" } else { "Nächstes Wochenende" },
                saturday,
                saturday + Days::new(1),
            ))
            .spacing(16)
            .width(Length::Fill)
            .into()
    }

    fn view_row<'a>(
        &'a self,
        manager: &'a Manager,
        now: NaiveDate,
        name: &'a str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Option<iced::Element<'a, Message>> {
        let elements = manager
            .events_between(from, to, self.filter.as_ref())
            .map(|item| self.view_event(item, now))
            .collect::<Vec<_>>();

        if !elements.is_empty() {
            let row = row![
                text(name).width(Length::FillPortion(1)).align_x(Alignment::End),
                column![].spacing(8).width(Length::FillPortion(1)).extend(elements),
            ].spacing(8).into();

            Some(row)
        } else {
            None
        }
    }

    fn view_event<'a>(
        &'a self,
        (calendar, _, event): (&'a Calendar, &NaiveDateTime, &'a calendar::Event),
        now: NaiveDate,
    ) -> iced::Element<'a, Message> {
        let icon = fa_icon_solid("circle").color(calendar.color).size(8.0);

        row![
            container(icon).padding(Padding::ZERO.top(6.0)),
            text(if self.should_skip(&calendar.uid) {
                event.summary.clone()
            } else {
                self.oneliner(event, now)
            }),
        ]
        .spacing(8)
        .into()
    }

    // @TODO: start and end times for rrules are out of place
    fn oneliner(&self, event: &calendar::Event, now: NaiveDate) -> String {
        let start = event.start_tz();
        let end = event.end_tz();

        println!("{}", event.start);

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

    fn current_saturday(&self, date: NaiveDate) -> NaiveDate {
        if date.weekday() == Weekday::Sat {
            return date;
        }

        if date.weekday() == Weekday::Sun {
            return date - Days::new(1);
        }

        date + Days::new(5 - date.weekday().num_days_from_monday() as u64)
    }

    fn should_skip(&self, uid: &Uuid) -> bool {
        self.config.as_ref().is_some_and(|c| c.skip_oneliner.contains(uid))
    }
}
