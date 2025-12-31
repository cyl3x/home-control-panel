use std::collections::BTreeMap;

use chrono::{Datelike, Duration, Locale, NaiveDate};

use crate::calendar::Manager;
use crate::config::{Config, UuidFilter};
use crate::messaging::{self, CalendarMessage};
use crate::prelude::*;
use crate::widgets::calendar::Dates;
use crate::widgets::calendar::month::grid_day::GridDayWidget;

mod grid_day;

pub type IndicatorMap = BTreeMap<NaiveDate, BTreeMap<uuid::Uuid, String>>;

const DURATION: chrono::TimeDelta = Duration::days(41);

pub struct MonthWidget {
    wrapper: gtk::Box,

    filter: Option<UuidFilter>,

    label: gtk::Label,
    grid: [GridDayWidget; 42],
    grid_start: NaiveDate,
}

impl MonthWidget {
    pub fn new(config: &Config, dates: &Dates) -> Self {
        let prev_button = gtk::Button::new();
        prev_button.add_css_class("calendar-month__control");
        prev_button.set_label("◀");
        prev_button.set_height_request(44);
        prev_button.set_width_request(44);
        prev_button.connect_clicked(|_| messaging::send_message(CalendarMessage::MonthPrev));

        let next_button = gtk::Button::new();
        next_button.add_css_class("calendar-month__control");
        next_button.set_label("▶");
        next_button.set_height_request(44);
        next_button.set_width_request(44);
        next_button.connect_clicked(|_| messaging::send_message(CalendarMessage::MonthNext));

        let label = gtk::Label::new(None);
        label.add_css_class("calendar-month__label");
        label.set_hexpand(false);

        let control_wrapper = gtk::CenterBox::new();
        control_wrapper.set_orientation(gtk::Orientation::Horizontal);
        control_wrapper.set_hexpand(true);
        control_wrapper.set_start_widget(Some(&prev_button));
        control_wrapper.set_center_widget(Some(&label));
        control_wrapper.set_end_widget(Some(&next_button));

        let weekdays = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        weekdays.set_hexpand(true);

        for day in ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"] {
            let label = gtk::Label::new(Some(day));
            label.add_css_class("calendar-month__weekday-label");
            label.set_halign(gtk::Align::Center);
            label.set_hexpand(true);

            weekdays.append(&label);
        }

        let month_grid = gtk::Grid::builder()
            .vexpand(true)
            .hexpand(true)
            .column_homogeneous(true)
            .row_homogeneous(true)
            .css_classes(["calendar-month__grid"])
            .row_spacing(8)
            .column_spacing(8)
            .build();

        let mut grid_elements = Vec::new();
        for row_idx in 0..6 {
            for column_idx in 0..7 {
                let day = GridDayWidget::new(row_idx * 7 + column_idx);

                month_grid.attach(day.widget(), column_idx as i32, row_idx as i32 + 1, 1, 1);

                grid_elements.push(day);
            }
        }

        let wrapper = gtk::Box::new(gtk::Orientation::Vertical, 0);
        wrapper.add_css_class("calendar-month");
        wrapper.set_vexpand(false);
        wrapper.set_valign(gtk::Align::Start);
        wrapper.append(&control_wrapper);
        wrapper.append(&weekdays);
        wrapper.append(&month_grid);

        Self {
            filter: config.calendar.month.clone(),
            label,
            wrapper,
            grid: grid_elements.try_into().expect("Month grid has wrong size"),
            grid_start: start_grid_date(dates.selected),
        }
    }

    pub const fn widget(&self) -> &gtk::Box {
        &self.wrapper
    }

    pub fn update_calendar(&mut self, manager: &Manager, dates: &Dates) {
        self.grid_start = start_grid_date(dates.selected);
        self.label.set_label(
            &dates
                .selected
                .format_localized("%B %Y", Locale::de_DE)
                .to_string(),
        );

        let indicators = manager
            .calendars_between(
                self.grid_start,
                self.grid_start + DURATION,
                self.filter.as_ref(),
            )
            .fold(
                BTreeMap::new(),
                |mut map: IndicatorMap, (event_start, calendar)| {
                    map.entry(event_start.date())
                        .or_default()
                        .entry(calendar.uid)
                        .or_insert_with(|| calendar.css_color());

                    map
                },
            );

        for grid_day in &mut self.grid {
            grid_day.update_calendar(&indicators, dates, self.grid_start);
        }
    }
}

pub fn start_grid_date(date: NaiveDate) -> NaiveDate {
    let mut first = date.with_day(1).unwrap();

    while first.weekday() != chrono::Weekday::Mon {
        first = first.pred_opt().unwrap_or(first);
    }

    first
}
