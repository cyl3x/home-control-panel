use std::collections::BTreeMap;

use crate::calendar::Calendar;
use crate::config::{Config, UuidFilter};
use crate::{messaging, prelude::*};

pub struct SelectionWidget {
    filter: Option<UuidFilter>,
    wrapper: gtk::Box,
    buttons: BTreeMap<uuid::Uuid, gtk::Button>,
}

impl SelectionWidget {
    pub fn new(config: &Config) -> Self {
        let wrapper = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        wrapper.add_css_class("calendar-selection");

        Self {
            filter: config.calendar.selection.clone(),
            wrapper,
            buttons: BTreeMap::new(),
        }
    }

    pub const fn widget(&self) -> &gtk::Box {
        &self.wrapper
    }

    pub fn update_calendar(&mut self, manager: &crate::calendar::Manager) {
        let calendars: BTreeMap<_, _> = manager.calendars(self.filter.as_ref()).collect();

        for (uid, button) in std::mem::take(&mut self.buttons) {
            if calendars.contains_key(&uid) {
                self.buttons.insert(uid, button);
            } else {
                self.wrapper.remove(&button);
            }
        }

        for (uid, (enabled, calendar)) in manager.calendars(self.filter.as_ref()) {
            if !self.buttons.contains_key(uid) {
                let button = create_button(calendar);

                self.wrapper.append(&button);
                self.buttons.insert(calendar.uid, button);
            }

            let button = self.buttons.get(uid).expect("Button just inserted");

            if *enabled {
                button.remove_css_class("disabled");
            } else {
                button.add_css_class("disabled");
            }
        }
    }
}

fn create_button(calendar: &Calendar) -> gtk::Button {
    let uid = calendar.uid;

    let label = gtk::Label::new(Some(&calendar.name));
    label.add_css_class("calendar-selection__button__label");
    label.set_hexpand(true);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let button = gtk::Button::new();
    button.add_css_class("calendar-selection__button");
    button.set_child(Some(&label));
    button.set_hexpand(true);
    button.inline_css(&format!("border-color: {};", calendar.css_color()));
    button.connect_clicked(move |_| {
        messaging::send_message(messaging::CalendarMessage::ToggleCalendar(uid));
    });

    button
}
