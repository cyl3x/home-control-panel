use chrono::{NaiveDateTime, TimeDelta, Timelike};
use gtk::glib;

use crate::calendar::Calendar;
use crate::calendar::Event;

/// Shown in place of the times, in two lines to fit their column
const WHOLE_DAYS: &str = "ganzer\nTag";

use glib::Object;

mod imp {
    use std::cell::{Cell, RefCell};

    use glib::Properties;
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;

    // Object holding the state
    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::EventObject)]
    pub struct EventObject {
        #[property(get, set)]
        summary: RefCell<String>,
        #[property(get, set)]
        description: RefCell<String>,
        #[property(get, set)]
        color: RefCell<String>,
        /// Start time, empty for events that last whole days
        #[property(get, set)]
        start: RefCell<String>,
        /// End time, empty when it would not say anything
        #[property(get, set)]
        end: RefCell<String>,
        /// Set while the event is running, which marks it in the list
        #[property(get, set)]
        running: Cell<bool>,
        /// Set once the event is over, which lets it step back
        #[property(get, set)]
        past: Cell<bool>,
        /// Set for events that last whole days, they get a compact row without times
        #[property(get, set)]
        whole_days: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EventObject {
        const NAME: &'static str = "EventObject";
        type Type = super::EventObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for EventObject {}
}

glib::wrapper! {
    pub struct EventObject(ObjectSubclass<imp::EventObject>);
}

impl EventObject {
    /// `occurrence` is the start of this occurrence, which differs from the event's own
    /// start for everything that repeats.
    pub fn new(calendar: &Calendar, event: &Event, occurrence: NaiveDateTime) -> Self {
        let times = Times::of(event, occurrence);

        Object::builder()
            .property("summary", &event.summary)
            .property("description", event.description.as_deref().unwrap_or(""))
            .property("color", calendar.css_color())
            .property("start", &times.start)
            .property("end", &times.end)
            .property("running", times.running)
            .property("past", times.past)
            .property("whole-days", times.whole_days)
            .build()
    }

    pub fn update(&self, calendar: &Calendar, event: &Event, occurrence: NaiveDateTime) {
        let times = Times::of(event, occurrence);

        self.set_summary(&*event.summary);
        self.set_description(event.description.as_deref().unwrap_or(""));
        self.set_color(calendar.css_color());
        self.set_start(times.start);
        self.set_end(times.end);
        self.set_running(times.running);
        self.set_past(times.past);
        self.set_whole_days(times.whole_days);
    }
}

/// What the list shows about when an event happens.
struct Times {
    start: String,
    end: String,
    running: bool,
    past: bool,
    whole_days: bool,
}

impl Times {
    fn of(event: &Event, occurrence: NaiveDateTime) -> Self {
        let duration = event.end - event.start;
        let end = occurrence + duration;

        // Whole-day events start at midnight and last full days, as in the agenda
        let whole_days = occurrence.and_utc().hour() == 0
            && duration.num_days() * 86400 == duration.num_seconds();

        let now = chrono::Local::now().naive_utc();

        Self {
            start: if whole_days {
                WHOLE_DAYS.to_string()
            } else {
                format_time(occurrence)
            },
            // An end at the same minute as the start says nothing
            end: if whole_days || end <= occurrence + TimeDelta::minutes(1) {
                String::new()
            } else {
                format_time(end)
            },
            running: !whole_days && occurrence <= now && now < end,
            past: end <= now,
            whole_days,
        }
    }
}

fn format_time(datetime: NaiveDateTime) -> String {
    datetime
        .and_utc()
        .with_timezone(&chrono::Local)
        .format("%H:%M")
        .to_string()
}
