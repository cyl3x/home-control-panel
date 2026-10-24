use chrono::NaiveDate;
use gtk::gio;
use gtk::glib;

use crate::calendar::Manager;
use crate::config::{Config, UuidFilter};
use crate::prelude::*;
use crate::widgets::calendar::Dates;
use crate::widgets::calendar::day::event_object::EventObject;

mod event_object;

/// The events of the selected day, with a header that names the day and counts them.
pub struct DayWidget {
    filter: Option<UuidFilter>,
    wrapper: gtk::Box,
    title: gtk::Label,
    count: gtk::Label,
    list: gtk::ListView,
    scrolled_to: Option<(NaiveDate, u32)>,
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
                .expect("The child has to be a Box");

            bind(&event, &item);
        });

        let list = gtk::ListView::new(Some(gtk::NoSelection::new(Some(store))), Some(factory));
        list.set_expand(true);

        let scrolled = gtk::ScrolledWindow::new();
        scrolled.set_expand(true);
        scrolled.set_child(Some(&list));
        scrolled.add_css_class("calendar-day");

        let title = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .hexpand(true)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(["calendar-day__header__title"])
            .build();

        let count = gtk::Label::builder()
            .halign(gtk::Align::End)
            .valign(gtk::Align::Baseline)
            .css_classes(["calendar-day__header__count"])
            .build();

        let header = gtk::Box::new(gtk::Orientation::Horizontal, 16);
        header.add_css_class("calendar-day__header");
        header.append(&title);
        header.append(&count);

        let wrapper = gtk::Box::new(gtk::Orientation::Vertical, 0);
        wrapper.set_expand(true);
        wrapper.append(&header);
        wrapper.append(&scrolled);

        Self {
            filter: config.calendar.day.clone(),
            wrapper,
            title,
            count,
            list,
            scrolled_to: None,
        }
    }

    pub const fn widget(&self) -> &gtk::Box {
        &self.wrapper
    }

    pub fn update_calendar(&mut self, manager: &Manager, dates: &Dates) {
        let mut len = 0;
        for (idx, (calendar, occurrence, event)) in manager
            .events_between(dates.selected, dates.selected, self.filter.as_ref())
            .enumerate()
        {
            len += 1;

            match self.model().item(idx as u32) {
                Some(item) => {
                    let item = item
                        .downcast::<EventObject>()
                        .expect("The item has to be an EventObject");
                    item.update(calendar, event, *occurrence);
                }
                None => {
                    self.store()
                        .append(&EventObject::new(calendar, event, *occurrence));
                }
            }
        }

        match len {
            0 => {
                self.store().remove_all();
            }
            // In one go, removing one by one shifts the indices
            len if len < self.model().n_items() => {
                let surplus = self.model().n_items() - len;
                self.store().splice(len, surplus, &[] as &[EventObject]);
            }
            _ => (),
        }

        self.title.set_label(&day_title(dates));
        self.count.set_label(&match len {
            0 => "Keine Termine".to_string(),
            1 => "1 Termin".to_string(),
            len => format!("{len} Termine"),
        });

        self.scroll_to_next(dates.selected);
    }

    /// Scroll to the first event that isn't over, but leave scrolling by hand alone.
    fn scroll_to_next(&mut self, date: NaiveDate) {
        let model = self.model();
        let next = (0..model.n_items())
            .find(|idx| {
                model
                    .item(*idx)
                    .and_downcast::<EventObject>()
                    .is_some_and(|event| !event.past())
            })
            .unwrap_or(0);

        if self.scrolled_to == Some((date, next)) {
            return;
        }

        self.scrolled_to = Some((date, next));

        if model.n_items() > 0 {
            self.list.scroll_to(next, gtk::ListScrollFlags::NONE, None);
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

/// "Heute, Do 22. Oktober", so the list says which day it belongs to.
fn day_title(dates: &Dates) -> String {
    let date = dates
        .selected
        .format_localized("%a %-d. %B", chrono::Locale::de_DE);

    if dates.is_today(dates.selected) {
        format!("Heute, {date}")
    } else {
        date.to_string()
    }
}

fn bind(event: &EventObject, item: &gtk::Box) {
    let times = item
        .first_child()
        .and_downcast::<gtk::Box>()
        .expect("The first child has to be the times");

    let start = times
        .first_child()
        .and_downcast::<gtk::Label>()
        .expect("The first time has to be a Label");

    let end = times
        .last_child()
        .and_downcast::<gtk::Label>()
        .expect("The last time has to be a Label");

    let indicator = times.next_sibling().expect("Needs to have the indicator");

    let labels = indicator
        .next_sibling()
        .and_downcast::<gtk::Box>()
        .expect("The labels have to be a Box");

    let summary = labels
        .first_child()
        .and_downcast::<gtk::Label>()
        .expect("The first child has to be a Label");

    let description = labels
        .last_child()
        .and_downcast::<gtk::Label>()
        .expect("The last child has to be a Label");

    let chip = item
        .last_child()
        .and_downcast::<gtk::Label>()
        .expect("The last child has to be the chip");

    for (property, label) in [
        ("summary", &summary),
        ("description", &description),
        ("start", &start),
        ("end", &end),
    ] {
        event
            .bind_property(property, label, "label")
            .sync_create()
            .build();
    }

    event
        .bind_property("start", &start, "visible")
        .transform_to(|_, text: String| Some(!text.is_empty()))
        .sync_create()
        .build();

    for (property, label) in [("end", &end), ("description", &description)] {
        let sync = glib::clone!(
            #[strong]
            label,
            move |event: &EventObject| {
                let text = event.property::<String>(property);
                label.set_visible(!text.is_empty() && !event.past());
            }
        );

        sync(event);

        for notify in [property, "past"] {
            event.connect_notify_local(Some(notify), {
                let sync = sync.clone();
                move |event, _| sync(event)
            });
        }
    }

    event
        .bind_property("running", &chip, "visible")
        .sync_create()
        .build();

    event.connect_notify_local(
        Some("color"),
        glib::clone!(
            #[strong]
            indicator,
            move |event, _| {
                indicator.inline_css(&format!("background-color: {}", event.color()));
            }
        ),
    );

    for (property, class) in [("past", "past"), ("whole-days", "allday")] {
        event.connect_notify_local(
            Some(property),
            glib::clone!(
                #[strong]
                item,
                move |event: &EventObject, _| {
                    let active = event.property_value(property).get::<bool>().unwrap_or(false);

                    item.set_class_active(class, active);
                }
            ),
        );
    }

    // Notifies are what carry the state onto the widgets, so replay them for a recycled row
    for property in ["color", "past", "whole-days"] {
        event.notify(property);
    }
}

fn create_day_event() -> gtk::Box {
    let start = gtk::Label::new(None);
    start.add_css_class("calendar-day__item__time");
    start.set_halign(gtk::Align::End);
    start.set_justify(gtk::Justification::Right);

    let end = gtk::Label::new(None);
    end.add_css_class("calendar-day__item__time__end");
    end.set_halign(gtk::Align::End);

    let times = gtk::Box::new(gtk::Orientation::Vertical, 0);
    times.set_width_request(64);
    times.set_valign(gtk::Align::Center);
    times.set_margin_start(20);
    times.append(&start);
    times.append(&end);

    let indicator = gtk::Box::new(gtk::Orientation::Vertical, 0);
    indicator.add_css_class("calendar-day__item__indicator");
    indicator.set_hexpand(false);
    indicator.set_vexpand(true);
    indicator.set_margin_vertical(12);
    indicator.set_margin_start(16);

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

    let label_wrapper = gtk::Box::new(gtk::Orientation::Vertical, 2);
    label_wrapper.add_css_class("calendar-day__item__labels");
    label_wrapper.set_hexpand(true);
    label_wrapper.set_valign(gtk::Align::Center);
    label_wrapper.append(&summary);
    label_wrapper.append(&description);

    let chip = gtk::Label::new(Some("Läuft"));
    chip.add_css_class("calendar-day__item__chip");
    chip.set_valign(gtk::Align::Center);
    chip.set_visible(false);

    let wrapper = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    wrapper.add_css_class("calendar-day__item");
    wrapper.append(&times);
    wrapper.append(&indicator);
    wrapper.append(&label_wrapper);
    wrapper.append(&chip);

    wrapper
}
