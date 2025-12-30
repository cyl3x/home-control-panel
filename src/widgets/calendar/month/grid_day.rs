use std::collections::BTreeMap;

use chrono::{Datelike, Duration, NaiveDate};

use crate::messaging;
use crate::prelude::*;
use crate::widgets::calendar::Dates;

#[derive(Debug)]
pub struct GridDayWidget {
    idx: usize,
    wrapper: gtk::Box,
    label: gtk::Label,
    indicator_wrapper: gtk::Box,
    indicators: BTreeMap<uuid::Uuid, gtk::Box>,
}

impl GridDayWidget {
    pub fn new(idx: usize) -> Self {
        let clickable = gtk::GestureClick::new();
        clickable.connect_pressed(move |controller, _, _, _| {
            if controller.current_button() == gtk::gdk::BUTTON_PRIMARY {
                messaging::send_message(messaging::CalendarMessage::SelectGridIndex(idx));
            }
        });

        let label = gtk::Label::new(Some("0"));
        label.set_expand(true);
        label.set_halign(gtk::Align::Center);
        label.set_valign(gtk::Align::End);
        label.add_css_class("calendar-month__grid-day__label");

        let indicator_wrapper = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        indicator_wrapper.set_valign(gtk::Align::Start);
        indicator_wrapper.set_vexpand(true);
        indicator_wrapper.add_css_class("calendar-month__grid-day__indicators");

        let indicator_center = gtk::CenterBox::new();
        indicator_center.set_expand(true);
        indicator_center.set_height_request(16); // height + margin
        indicator_center.set_width_request(12 * 3 + 4 * 3);
        indicator_center.set_center_widget(Some(&indicator_wrapper));

        let wrapper = gtk::Box::new(gtk::Orientation::Vertical, 0);
        wrapper.set_expand(true);
        wrapper.add_css_class("calendar-month__grid-day");
        wrapper.append(&label);
        wrapper.append(&indicator_center);
        wrapper.add_controller(clickable);

        Self {
            idx,
            wrapper,
            label,
            indicator_wrapper,
            indicators: BTreeMap::new(),
        }
    }

    pub const fn widget(&self) -> &gtk::Box {
        &self.wrapper
    }

    pub fn update_calendar(
        &mut self,
        indicator_map: &super::IndicatorMap,
        dates: &Dates,
        grid_start: NaiveDate,
    ) {
        let date = grid_start + Duration::days(self.idx as i64);
        self.label.set_label(&date.day().to_string());

        self.wrapper.remove_css_class("selected");
        self.wrapper.remove_css_class("today");
        self.wrapper.remove_css_class("not-month");

        if dates.is_selected(date) {
            self.wrapper.add_css_class("selected");
        } else if dates.is_today(date) {
            self.wrapper.add_css_class("today");
        } else if !dates.is_month(date) {
            self.wrapper.add_css_class("not-month");
        }

        let indicators = std::mem::take(&mut self.indicators);

        for (uid, indicator) in indicators {
            if indicator_map
                .get(&date)
                .is_none_or(|map| !map.contains_key(&uid))
            {
                self.indicator_wrapper.remove(&indicator);
            } else {
                self.indicators.insert(uid, indicator);
            }
        }

        for (uid, color) in indicator_map
            .get(&date)
            .map(|i| i.iter())
            .unwrap_or_default()
        {
            if !self.indicators.contains_key(uid) {
                let indicator = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                indicator.add_css_class("calendar-month__grid-day__indicator");
                indicator.set_width_request(12);
                indicator.set_height_request(12);
                indicator.inline_css(&format!("background-color: {color}"));

                self.indicator_wrapper.append(&indicator);
                self.indicators.insert(*uid, indicator);
            }
        }
    }
}
