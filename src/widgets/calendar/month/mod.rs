use chrono::{Datelike, Duration, Locale, NaiveDate};
use gtk::{AspectFrame, Box, Button, Constraint, Grid, Label, Widget};
use gtk::prelude::*;

use crate::calendar::Manager;
use crate::messaging::{self, AppMessage};
use crate::widgets::calendar::month::grid_day::GridDay;
use crate::widgets::calendar::{Dates, month};

mod grid_day;

pub struct Month {
    wrapper: Box,

    prev_button: Button,
    next_button: Button,
    month_label: Label,
    month_grid: [GridDay; 42],
}

impl Month {
    pub fn new() -> Self {
        let prev_button = Button::builder()
            .label("◀")
            .build();

        let next_button = Button::builder()
            .label("▶")
            .build();

        let month_label = Label::builder()
            .label("")
            .hexpand(true)
            .css_classes(["calendar-month-label"])
            .build();

        let control_wrapper = Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(true)
            .build();

        prev_button.connect_clicked(|_| {
            messaging::send_message(messaging::AppMessage::CalendarMonthPrev);
        });

        next_button.connect_clicked(|_| {
            messaging::send_message(messaging::AppMessage::CalendarMonthNext);
        });

        control_wrapper.append(&prev_button);
        control_wrapper.append(&month_label);
        control_wrapper.append(&next_button);

        let weekdays = Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(true)
            .build();

        for day in ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"] {
            let label = day_label(day);
            weekdays.append(&label);
        }

        let month_grid = Grid::builder()
            .vexpand(true)
            .hexpand(true)
            .valign(gtk::Align::Start)
            .column_homogeneous(true)
            .row_homogeneous(true)
            .css_classes(["calendar-month-grid"])
            .build();

        let mut month_grid_elements = Vec::new();
        for row_idx in 0..6 {
            for column_idx in 0..7 {
                let day = GridDay::new(row_idx * 7 + column_idx);

                month_grid.attach(day.widget(), column_idx as i32, row_idx as i32, 1, 1);

                month_grid_elements.push(day);
            }
        }

        let wrapper = Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .css_classes(["calendar-month"])
            .build();

        wrapper.append(&control_wrapper);
        wrapper.append(&weekdays);
        wrapper.append(&month_grid);

        Month {
            prev_button,
            next_button,
            month_label,
            wrapper,
            month_grid: month_grid_elements.try_into().expect("Month grid has wrong size"),
        }
    }

    pub fn widget(&self) -> &Box {
        &self.wrapper
    }

    pub fn update(&self, message: &AppMessage, dates: &Dates) {
        match message {
            AppMessage::CalendarMonthPrev => {
                messaging::send_message(AppMessage::CalendarSelectDate(dates.selected - chrono::Months::new(1)));
            },
            AppMessage::CalendarMonthNext => {
                messaging::send_message(AppMessage::CalendarSelectDate(dates.selected + chrono::Months::new(1)));
            }
            _ => (),
        }
    }

    pub fn update_calendar(&self, manager: &Manager, dates: &Dates) {

        self.month_label.set_label(&dates.selected.format_localized("%B %Y", Locale::de_DE).to_string());

        let start = start_grid_date(dates.selected);

        for grid_day in &self.month_grid {
            grid_day.update_calendar(manager, dates, start);
        }
    }
}

fn day_label(day: &str) -> Label {
    Label::builder()
        .label(day)
        .halign(gtk::Align::Center)
        .hexpand(true)
        .css_classes(["calendar-month-weekday-label"])
        .build()
}

fn start_grid_date(date: NaiveDate) -> NaiveDate {
    let mut first = date.with_day(1).unwrap();

    while first.weekday() != chrono::Weekday::Mon {
        first = first.pred_opt().unwrap_or(first);
    }

    first
}
