use std::time::{Duration, Instant};

use chrono::{Datelike, NaiveDate, NaiveDateTime};
use iced::widget::column;
use uuid::Uuid;

use crate::calendar::{CalendarMap, Manager};
use crate::config;

mod day;
mod event;
mod month;
mod selection;

pub struct Calendar {
    manager: Manager,
    dates: Dates,

    selection: selection::CalendarSelection,
    month: month::Month,
    day: day::Day,
    event: event::Event,
}

pub struct Dates {
    pub now: NaiveDateTime,
    pub selected: NaiveDate,
}

impl Dates {
    pub fn is_today(&self, date: NaiveDate) -> bool {
        date == self.now.date()
    }

    pub fn is_selected(&self, date: NaiveDate) -> bool {
        date == self.selected
    }

    pub fn is_month(&self, date: NaiveDate) -> bool {
        date.month() == self.selected.month()
    }
}

#[derive(Debug)]
pub enum Message {
    Sync,
    NextDay(Instant),
    SelectDate(NaiveDate),
    UpdateMap(Box<Option<CalendarMap>>),
    ToggleCalendar(Uuid),
}

impl Clone for Message {
    fn clone(&self) -> Self {
        match self {
            Self::UpdateMap(_) => panic!("UpdateMap should not be cloned"),
            Self::Sync => Self::Sync,
            Self::NextDay(instant) => Self::NextDay(*instant),
            Self::SelectDate(date) => Self::SelectDate(*date),
            Self::ToggleCalendar(uid) => Self::ToggleCalendar(*uid),
        }
    }
}

impl Calendar {
    pub fn new(ical: config::Ical, configs: config::Calendars) -> (Self, iced::Task<Message>) {
        let now = chrono::Utc::now().naive_utc();
        let mut calendar = Self {
            manager: Manager::new(ical),
            dates: Dates {
                now,
                selected: now.date(),
            },

            selection: selection::CalendarSelection::new(configs.selection),
            month: month::Month::new(configs.month),
            day: day::Day::new(configs.day),
            event: event::Event::new(configs.event),
        };

        let task = calendar.update(Message::Sync);

        (calendar, task)
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::Subscription::batch([
            iced::time::every(until_next_day(self.dates.now)).map(Message::NextDay),
            iced::time::every(Duration::from_secs(600)).map(|_| Message::Sync),
        ])
    }

    pub fn view(&self) -> iced::Element<Message> {
        column![
            self.month.view(&self.manager, &self.dates),
            self.selection.view(&self.manager),
            self.day.view(&self.manager, &self.dates),
            self.event.view(&self.manager, &self.dates),
        ]
        .padding(16)
        .spacing(16)
        .into()
    }

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::Sync => {
                let client = self.manager.client.clone();
                return iced::Task::perform(
                    async move {
                        client.get_map().map_or_else(
                            |e| {
                                log::error!("Failed to fetch calendar map: {:?}", e);
                                Box::new(None)
                            },
                            |map| {
                                log::info!(
                                    "Fetched calendar map: {} calendars, {} events",
                                    map.len_calendars(),
                                    map.len_events()
                                );
                                Box::new(Some(map))
                            },
                        )
                    },
                    Message::UpdateMap,
                );
            }
            Message::NextDay(_) => {
                self.dates.now = chrono::Utc::now().naive_utc();
                self.dates.selected = self.dates.now.date();
            }
            Message::UpdateMap(map) => {
                if let Some(map) = *map {
                    self.manager.set_map(map);
                }
            }
            Message::SelectDate(date) => {
                self.dates.selected = date;
            }
            Message::ToggleCalendar(uid) => {
                self.manager.toggle_calendar(uid);
            }
        };

        iced::Task::none()
    }
}

fn until_next_day(date: NaiveDateTime) -> Duration {
    let next_day = date
        .date()
        .succ_opt()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .unwrap();
    let secs = (next_day - date).num_seconds();

    Duration::from_secs(secs as u64 + 30)
}
