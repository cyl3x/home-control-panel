use gtk::gio;

use crate::calendar::Manager;
use crate::config::{Config, UuidFilter};
use crate::prelude::*;
use crate::widgets::calendar::Dates;
use crate::widgets::calendar::day::event_object::EventObject;

mod event_object;

pub struct DayWidget {
    filter: Option<UuidFilter>,
    wrapper: gtk::ScrolledWindow,
    list: gtk::ListView,
}

impl DayWidget {
    pub fn new(config: &Config) -> Self {
        let store = gio::ListStore::new::<EventObject>();

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");

            list_item.set_child(Some(&create_day_event()));
        });

        factory.connect_bind(move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            let event = list_item
                .item()
                .and_downcast::<EventObject>()
                .expect("The item has to be an EventObject");

            let item = list_item
                .child()
                .and_downcast::<gtk::Box>()
                .expect("The child has to be a Label");

            let indicator = item.first_child().expect("Needs to have the indicator");

            let label_wrapper = item
                .last_child()
                .and_downcast::<gtk::Box>()
                .expect("The last child has to be a Box");

            let summary = label_wrapper
                .first_child()
                .and_downcast::<gtk::Label>()
                .expect("The first child has to be a Label");

            let description = label_wrapper
                .last_child()
                .and_downcast::<gtk::Label>()
                .expect("The last child has to be a Label");

            event
                .bind_property("summary", &summary, "label")
                .sync_create()
                .build();

            event
                .bind_property("description", &description, "label")
                .sync_create()
                .build();

            event.connect_notify_local(Some("color"), move |event, _| {
                let color = event.property_value("color").get::<String>().unwrap();
                indicator.inline_css(&format!("background-color: {color}"));
            });

            event.set_property("color", event.property_value("color"));
        });

        let list = gtk::ListView::new(Some(gtk::NoSelection::new(Some(store))), Some(factory));
        list.set_expand(true);

        let wrapper = gtk::ScrolledWindow::new();
        wrapper.set_expand(true);
        wrapper.set_child(Some(&list));
        wrapper.add_css_class("calendar-day");

        Self {
            filter: config.calendar.day.clone(),
            wrapper,
            list,
        }
    }

    pub const fn widget(&self) -> &gtk::ScrolledWindow {
        &self.wrapper
    }

    pub fn update_calendar(&mut self, manager: &Manager, dates: &Dates) {
        let mut len = 0;
        for (idx, (calendar, _, event)) in manager
            .events_between(dates.selected, dates.selected, self.filter.as_ref())
            .enumerate()
        {
            len += 1;

            match self.model().item(idx as u32) {
                Some(item) => {
                    let item = item
                        .downcast::<EventObject>()
                        .expect("The item has to be an EventObject");
                    item.update(calendar, event);
                }
                None => {
                    self.store().append(&EventObject::new(calendar, event));
                }
            }
        }

        match len {
            0 => {
                self.store().remove_all();
            }
            len if len < self.model().n_items() => {
                for idx in len..self.model().n_items() {
                    self.store().remove(idx);
                }
            }
            _ => (),
        }
    }

    fn model(&self) -> gtk::NoSelection {
        self.list
            .model()
            .expect("Should have the selection")
            .downcast::<gtk::NoSelection>()
            .expect("Should be NoSelection")
    }

    fn store(&self) -> gio::ListStore {
        self.model()
            .model()
            .expect("Should have ListStore")
            .downcast::<gio::ListStore>()
            .expect("Should be ListStore")
    }
}

fn create_day_event() -> gtk::Box {
    let indicator = gtk::Box::new(gtk::Orientation::Vertical, 0);
    indicator.add_css_class("calendar-day__item__indicator");
    indicator.set_hexpand(false);
    indicator.set_vexpand(true);

    let summary = gtk::Label::new(None);
    summary.add_css_class("calendar-day__item__labels__summary");
    summary.set_hexpand(true);
    summary.set_halign(gtk::Align::Start);
    summary.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let description = gtk::Label::new(None);
    description.add_css_class("calendar-day__item__labels__description");
    description.set_hexpand(true);
    description.set_halign(gtk::Align::Start);
    description.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let label_wrapper = gtk::Box::new(gtk::Orientation::Vertical, 4);
    label_wrapper.add_css_class("calendar-day__item__labels");
    label_wrapper.set_hexpand(true);
    label_wrapper.append(&summary);
    label_wrapper.append(&description);

    let wrapper = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    wrapper.add_css_class("calendar-day__item");
    wrapper.append(&indicator);
    wrapper.append(&label_wrapper);

    wrapper
}
