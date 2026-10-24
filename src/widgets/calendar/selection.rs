use std::collections::BTreeMap;

use crate::calendar::Calendar;
use crate::config::{Config, UuidFilter};
use crate::{messaging, prelude::*};

const ICON_HIDDEN: char = '\u{f070}'; // eye-slash

pub struct SelectionWidget {
    filter: Option<UuidFilter>,
    wrapper: gtk::Box,
    buttons: BTreeMap<uuid::Uuid, (gtk::Button, gtk::Label)>,
}

impl SelectionWidget {
    pub fn new(config: &Config) -> Self {
        // Always one row: the names are shortened rather than wrapping, so the filters
        // keep a fixed height whatever the column is doing
        let wrapper = gtk::Box::new(gtk::Orientation::Horizontal, 6);
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

        for (uid, (button, hidden)) in std::mem::take(&mut self.buttons) {
            if calendars.contains_key(&uid) {
                self.buttons.insert(uid, (button, hidden));
            } else {
                self.wrapper.remove(&button);
            }
        }

        for (uid, (enabled, calendar)) in manager.calendars(self.filter.as_ref()) {
            if !self.buttons.contains_key(uid) {
                let (button, hidden) = create_button(calendar);

                self.wrapper.append(&button);
                self.buttons.insert(calendar.uid, (button, hidden));
            }

            let (button, hidden) = self.buttons.get(uid).expect("Button just inserted");

            button.set_class_active("off", !*enabled);
            hidden.set_visible(!*enabled);
        }
    }
}

fn create_button(calendar: &Calendar) -> (gtk::Button, gtk::Label) {
    let uid = calendar.uid;

    let hidden = gtk::Label::builder()
        .label(ICON_HIDDEN.to_string())
        .visible(false)
        .css_classes(["calendar-selection__button__hidden", "icon"])
        .build();

    let label = gtk::Label::new(Some(&calendar.name));
    label.add_css_class("calendar-selection__button__label");
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    content.set_halign(gtk::Align::Center);
    content.append(&hidden);
    content.append(&label);

    let button = gtk::Button::new();
    button.add_css_class("calendar-selection__button");
    button.set_child(Some(&content));
    button.set_hexpand(true);
    button.inline_css(&format!("border-color: {};", calendar.css_color()));
    button.connect_clicked(move |_| {
        messaging::send_message(messaging::CalendarMessage::ToggleCalendar(uid));
    });

    (button, hidden)
}
