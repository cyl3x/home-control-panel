use chrono::{Duration, NaiveDate, Datelike};
use gtk::{Box, Label};
use gtk::prelude::*;

use crate::calendar::Manager;
use crate::messaging;
use crate::widgets::calendar::Dates;

#[derive(Debug)]
pub struct GridDay {
    idx: usize,
    wrapper: Box,
    label: Label,
    indicators: Box,
}

impl GridDay {
    pub fn new(idx: usize) -> GridDay {
        let label = Label::builder()
            .label("0")
            .hexpand(true)
            .vexpand(true)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .css_classes(["calendar-month__grid-day__label"])
            .build();

        let indicators = Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(true)
            .spacing(4)
            .css_classes(["calendar-month__grid-day__indicators"])
            .build();

        let wrapper = Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .hexpand(true)
            .vexpand(true)
            .css_classes(["calendar-month__grid-day"])
            .build();

        wrapper.append(&label);
        wrapper.append(&indicators);

        wrapper.connect_focus_on_click_notify(move |_| {
            messaging::send_message(messaging::AppMessage::CalendarSelectIndex(idx));
        });

        GridDay {
            idx,
            wrapper,
            label,
            indicators,
        }
    }

    pub fn widget(&self) -> &Box {
        &self.wrapper
    }

    pub fn update_calendar(&self, manager: &Manager, dates: &Dates, grid_start: NaiveDate) {
        let date = grid_start + Duration::days(self.idx as i64);
        self.label.set_label(&date.day().to_string());

        self.wrapper.remove_css_class("selected");
        self.wrapper.remove_css_class("today");


        if dates.is_today(date) {
            self.wrapper.add_css_class("today");
        } else if dates.is_selected(date) {
            self.wrapper.add_css_class("selected");
        }
    }
}
