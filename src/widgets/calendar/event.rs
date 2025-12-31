use crate::calendar::Manager;
use crate::config::{Config, UuidFilter};
use crate::prelude::*;
use crate::widgets::calendar::Dates;

pub struct EventWidget {
    filter: Option<UuidFilter>,

    wrapper: gtk::Box,
    label: gtk::Label,
    indicator: gtk::Box,
}

impl EventWidget {
    pub fn new(config: &Config) -> Self {
        let indicator = gtk::Box::new(gtk::Orientation::Vertical, 0);
        indicator.add_css_class("calendar-event__indicator");
        indicator.set_vexpand(true);
        indicator.set_hexpand(false);

        let label = gtk::Label::new(None);
        label.add_css_class("calendar-event__label");
        label.set_hexpand(true);
        label.set_halign(gtk::Align::Start);

        let wrapper = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        wrapper.add_css_class("calendar-event");
        wrapper.set_valign(gtk::Align::End);
        wrapper.append(&indicator);
        wrapper.append(&label);

        Self {
            filter: config.calendar.event.clone(),
            wrapper,
            label,
            indicator,
        }
    }

    pub const fn widget(&self) -> &gtk::Box {
        &self.wrapper
    }

    pub fn update_calendar(&mut self, manager: &Manager, dates: &Dates) {
        let event = manager
            .events_between(dates.now.naive_utc().date(), dates.now.naive_utc().date(), self.filter.as_ref())
            .next();

        if let Some((calendar, _, event)) = event {
            self.label.set_label(&event.summary);
            self.indicator
                .inline_css(&format!("background-color: {}", calendar.css_color()));
        }

        self.wrapper.set_visible(event.is_some());
    }
}
