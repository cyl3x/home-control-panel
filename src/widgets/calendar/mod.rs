use chrono::{Duration, NaiveDate};
use gtk::glib;

use crate::messaging::{self, CalendarMessage};
use crate::prelude::*;
use crate::widgets::calendar::day::DayWidget;
use crate::widgets::calendar::event::EventWidget;
use crate::widgets::calendar::month::start_grid_date;
use crate::widgets::calendar::selection::SelectionWidget;
use crate::widgets::calendar::upcoming::UpcomingWidget;
use crate::{calendar::Manager, config::Config, widgets::calendar::month::MonthWidget};

pub mod day;
pub mod event;
pub mod month;
pub mod selection;
pub mod upcoming;

pub struct Dates {
    pub now: DateTime<Local>,
    pub selected: NaiveDate,
}

impl Dates {
    pub fn today(&self) -> NaiveDate {
        self.now.naive_local().date()
    }

    pub fn is_today(&self, date: NaiveDate) -> bool {
        date == self.now.date_naive()
    }

    pub fn is_selected(&self, date: NaiveDate) -> bool {
        date == self.selected
    }

    pub fn is_month(&self, date: NaiveDate) -> bool {
        date.month() == self.selected.month()
    }
}

pub struct CalendarWidget {
    wrapper: gtk::Box,
    month: MonthWidget,
    day: DayWidget,
    selection: SelectionWidget,
    event: EventWidget,
    upcoming: UpcomingWidget,

    dates: Dates,
    manager: Manager,
    reset_dates_timeout: Option<glib::SourceId>,
    next_day_timeout: Option<glib::SourceId>,
}

impl CalendarWidget {
    pub fn new(config: &Config) -> Self {
        let dates = Dates {
            now: chrono::Local::now(),
            selected: chrono::Local::now().naive_local().date(),
        };

        let month = MonthWidget::new(config, &dates);
        let day = DayWidget::new(config);
        let selection = SelectionWidget::new(config);
        let event = EventWidget::new(config);
        let upcoming = UpcomingWidget::new(config);

        let wrapper = gtk::Box::new(gtk::Orientation::Vertical, 16);
        wrapper.append(month.widget());
        wrapper.append(selection.widget());
        wrapper.append(day.widget());
        wrapper.append(event.widget());

        glib::timeout_add_seconds(600, move || {
            messaging::send_message(CalendarMessage::Fetch);

            glib::ControlFlow::Continue
        });

        messaging::send_message(CalendarMessage::SelectNow);
        messaging::send_message(CalendarMessage::Fetch);

        Self {
            wrapper,
            month,
            day,
            selection,
            event,
            upcoming,

            manager: Manager::new(config.ical.clone()),
            reset_dates_timeout: None,
            next_day_timeout: None,
            dates,
        }
    }

    pub const fn widget(&self) -> &gtk::Box {
        &self.wrapper
    }

    pub const fn upcoming(&self) -> &UpcomingWidget {
        &self.upcoming
    }

    pub fn update(&mut self, message: CalendarMessage) {
        match message {
            CalendarMessage::Fetch => {
                let client = self.manager.client.clone();

                log::info!("Calendar: fetching calendar map");

                gtk::gio::spawn_blocking(move || match client.get_map() {
                    Err(err) => log::error!("Calendar: failed to fetch map: {err:?}"),
                    Ok(map) => {
                        log::info!(
                            "Calendar: fetched map: {} calendars, {} events",
                            map.len_calendars(),
                            map.len_events()
                        );

                        messaging::send_message(CalendarMessage::UpdateMap(std::boxed::Box::new(
                            map,
                        )));
                    }
                });
            }
            CalendarMessage::UpdateMap(map) => {
                if self.manager.set_map(*map) {
                    log::info!("Calendar: map changed and updated");

                    self.update_calendar();
                }
            }
            CalendarMessage::SelectNow => {
                self.dates.now = chrono::Local::now();
                self.dates.selected = self.dates.now.date_naive();

                log::info!("Calendar: selected now {}", self.dates.now);

                remove_source(self.reset_dates_timeout.take());
                self.update_calendar();
                self.next_day_timeout();
            }
            CalendarMessage::SelectDate(date) => {
                self.dates.selected = date;
                self.reset_dates_timeout();

                self.update_calendar();
            }
            CalendarMessage::ToggleCalendar(uid) => {
                self.manager.toggle_calendar(uid);

                log::info!("Calendar: toggled \"{}\"", self.manager.calendar_name(&uid).unwrap_or_else(|| uid.to_string()));

                self.update_calendar();
            }
            CalendarMessage::SelectGridIndex(idx) => {
                let selected_date = start_grid_date(self.dates.selected) + Duration::days(idx as i64);

                log::info!("Calendar: selected date {}", selected_date);

                messaging::send_message(CalendarMessage::SelectDate(selected_date));
            }
            CalendarMessage::MonthPrev => messaging::send_message(CalendarMessage::SelectDate(
                self.dates.selected - chrono::Months::new(1),
            )),
            CalendarMessage::MonthNext => messaging::send_message(CalendarMessage::SelectDate(
                self.dates.selected + chrono::Months::new(1),
            )),
        }
    }

    fn update_calendar(&mut self) {
        self.month.update_calendar(&self.manager, &self.dates);
        self.day.update_calendar(&self.manager, &self.dates);
        self.selection.update_calendar(&self.manager);
        self.event.update_calendar(&self.manager, &self.dates);
        self.upcoming.update_calendar(&self.manager, &self.dates);

        log::info!("Calendar: updated for date {}", self.dates.selected);
    }

    fn reset_dates_timeout(&mut self) {
        remove_source(self.reset_dates_timeout.take());

        self.reset_dates_timeout = Some(glib::timeout_add_seconds_once(60, || {
            messaging::send_message(CalendarMessage::SelectNow);
        }));
    }

    fn next_day_timeout(&mut self) {
        remove_source(self.next_day_timeout.take());

        let now = self.dates.now.naive_local();
        let next_day = now
            .date()
            .succ_opt()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .unwrap();

        let seconds = (next_day - now).num_seconds() as u32 + 30;

        self.next_day_timeout = Some(glib::timeout_add_seconds_once(seconds, move || {
            messaging::send_message(CalendarMessage::SelectNow);
        }));
    }
}

