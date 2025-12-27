use chrono::{Duration, NaiveDate, NaiveDateTime};
use gtk::glib::SourceId;
use gtk::{Box, glib};
use gtk::prelude::*;
use chrono::prelude::*;

use crate::messaging::{self, AppMessage};
use crate::{calendar::Manager, config::Config, widgets::calendar::month::Month};

pub mod month;

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

pub struct Calendar {
    wrapper: Box,

    manager: Manager,
    month: Month,

    reset_dates: Option<SourceId>,

    dates: Dates,
}

impl Calendar {
    pub fn new(config: &Config) -> Self {
        let dates = Dates {
            now: chrono::Utc::now().naive_utc(),
            selected: chrono::Utc::now().naive_utc().date(),
        };

        let month = Month::new();

        let wrapper = Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        wrapper.append(month.widget());

        next_day_timeout(dates.now);
        calendar_sync_timeout();

        messaging::send_message(AppMessage::CalendarSelectNow);
        messaging::send_message(AppMessage::CalendarFetch);

        Calendar {
            wrapper,

            manager: Manager::new(config.ical.clone()),
            month,

            reset_dates: None,

            dates,
        }
    }

    pub fn widget(&self) -> &Box {
        &self.wrapper
    }

    pub fn update(&mut self, message: AppMessage) {
        match message {
            AppMessage::CalendarFetch => {
                let client = self.manager.client.clone();

                gtk::gio::spawn_blocking(move || {
                    match client.clone().get_map() {
                        Err(err) => log::error!("Failed to fetch calendar map: {err:?}"),
                        Ok(map) => messaging::send_message(AppMessage::CalendarUpdateMap(std::boxed::Box::new(map))),
                    }
                });
            }
            AppMessage::CalendarUpdateMap(map) => {
                log::info!(
                    "Fetched calendar map: {} calendars, {} events",
                    map.len_calendars(),
                    map.len_events()
                );

                if self.manager.set_map(*map) {
                    self.update_calendar();
                }
            }
            AppMessage::CalendarSelectNow => {
                self.dates.now = chrono::Utc::now().naive_utc();
                self.dates.selected = self.dates.now.date();
                if let Some(id) = self.reset_dates.take() {
                    id.remove();
                }

                log::info!("Next day: {}", self.dates.now);

                self.update_calendar();
            }
            AppMessage::CalendarSelectDate(date) => {
                self.dates.selected = date;
                self.reset_dates_timeout();

                self.update_calendar();
            }
            AppMessage::CalendarSelectIndex(idx) => {
                self.dates.selected = (self.dates.now + Duration::days(idx as i64)).date();
                self.reset_dates_timeout();

                self.update_calendar();
            }
            AppMessage::CalendarToggleCalendar(uid) => {
                self.manager.toggle_calendar(uid);

                self.update_calendar();
            }
            AppMessage::CalendarMonthPrev => messaging::send_message(AppMessage::CalendarSelectDate(self.dates.selected - chrono::Months::new(1))),
            AppMessage::CalendarMonthNext => messaging::send_message(AppMessage::CalendarSelectDate(self.dates.selected + chrono::Months::new(1))),
            _ => ()
        }
    }

    fn update_calendar(&self) {
        self.month.update_calendar(&self.manager, &self.dates);
    }

    fn reset_dates_timeout(&mut self) {
        if let Some(id) = self.reset_dates.take() {
            id.remove();
        }

        self.reset_dates = Some(glib::timeout_add_seconds_once(60, || {
            messaging::send_message(AppMessage::CalendarSelectNow);
        }));
    }
}

fn calendar_sync_timeout() {
    glib::timeout_add_seconds(600, move || {
        messaging::send_message(AppMessage::CalendarFetch);

        glib::ControlFlow::Continue
    });
}

fn next_day_timeout(now: NaiveDateTime) {
    let next_day = now
        .date()
        .succ_opt()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .unwrap();

    let seconds = (next_day - now).num_seconds() as u32 + 30;

    glib::timeout_add_seconds_once(seconds, move || {
        messaging::send_message(AppMessage::CalendarSelectNow);

        next_day_timeout(chrono::Utc::now().naive_utc());
    });
}
