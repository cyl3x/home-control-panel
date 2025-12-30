use chrono::{Duration, NaiveDate, NaiveDateTime};
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

pub struct CalendarWidget {
    wrapper: gtk::Box,
    month: MonthWidget,
    day: DayWidget,
    selection: SelectionWidget,
    event: EventWidget,
    upcoming: UpcomingWidget,

    manager: Manager,
    reset_dates: Option<glib::SourceId>,
    dates: Dates,
}

impl CalendarWidget {
    pub fn new(config: &Config) -> Self {
        let dates = Dates {
            now: chrono::Utc::now().naive_utc(),
            selected: chrono::Utc::now().naive_utc().date(),
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

        next_day_timeout(dates.now);
        calendar_sync_timeout();

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
            reset_dates: None,
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

                gtk::gio::spawn_blocking(move || match client.get_map() {
                    Err(err) => log::error!("Failed to fetch calendar map: {err:?}"),
                    Ok(map) => {
                        log::info!(
                            "Fetched calendar map: {} calendars, {} events",
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
                    self.update_calendar();

                    log::info!("Calendar map updated");
                }
            }
            CalendarMessage::SelectNow => {
                self.dates.now = chrono::Utc::now().naive_utc();
                self.dates.selected = self.dates.now.date();
                log::info!("Next day: {}", self.dates.now);

                self.remove_reset_dates();
                self.update_calendar();
            }
            CalendarMessage::SelectDate(date) => {
                self.dates.selected = date;
                self.add_reset_dates();

                self.update_calendar();
            }
            CalendarMessage::ToggleCalendar(uid) => {
                self.manager.toggle_calendar(uid);

                self.update_calendar();
            }
            CalendarMessage::SelectGridIndex(idx) => {
                messaging::send_message(CalendarMessage::SelectDate(
                    start_grid_date(self.dates.selected) + Duration::days(idx as i64),
                ));
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
    }

    fn remove_reset_dates(&mut self) {
        if let Some(id) = self.reset_dates.take()
            && glib::MainContext::default()
                .find_source_by_id(&id)
                .is_some()
        {
            id.remove();
        }
    }

    fn add_reset_dates(&mut self) {
        self.remove_reset_dates();

        self.reset_dates = Some(glib::timeout_add_seconds_once(60, || {
            messaging::send_message(CalendarMessage::SelectNow);
        }));
    }
}

fn calendar_sync_timeout() {
    glib::timeout_add_seconds(600, move || {
        messaging::send_message(CalendarMessage::Fetch);

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
        messaging::send_message(CalendarMessage::SelectNow);

        next_day_timeout(chrono::Utc::now().naive_utc());
    });
}
