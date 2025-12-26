use gtk::glib;

use crate::calendar::Calendar;
use crate::calendar::Event;

use glib::Object;

mod imp {
    use std::cell::RefCell;

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
    pub fn new(calendar: &Calendar, event: &Event) -> Self {
        Object::builder()
            .property("summary", &event.summary)
            .property("description", event.description.as_deref().unwrap_or(""))
            .property("color", calendar.css_color())
            .build()
    }

    pub fn update(&self, calendar: &Calendar, event: &Event) {
        self.set_summary(&*event.summary);
        self.set_description(event.description.as_deref().unwrap_or(""));
        self.set_color(calendar.css_color());
    }
}
