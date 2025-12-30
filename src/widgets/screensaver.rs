use chrono::DateTime;
use chrono::Utc;

use crate::config;
use crate::config::Config;
use crate::messaging;
use crate::messaging::ScreensaverMessage;
use crate::prelude::*;
use crate::widgets::calendar::upcoming::UpcomingWidget;

pub struct ScreensaverWidget {
    now: DateTime<Utc>,
    last_activity: DateTime<Utc>,
    config: config::Screensaver,

    wrapper: gtk::Box,
    center_wrapper: gtk::Box,
    date: gtk::Label,
    time: gtk::Label,
}

impl ScreensaverWidget {
    pub fn new(config: &Config, upcoming: &UpcomingWidget) -> Self {
        let date = gtk::Label::builder()
            .css_classes(["screensaver__date"])
            .build();

        let time = gtk::Label::builder()
            .css_classes(["screensaver__time"])
            .build();

        let clock = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .build();

        clock.append(&date);
        clock.append(&time);

        let center_wrapper = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .hexpand(false)
            .vexpand(true)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .css_classes(["screensaver__center"])
            .build();

        center_wrapper.append(&clock);
        center_wrapper.append(upcoming.widget());

        let controller = gtk::GestureClick::new();
        controller.connect_pressed(move |controller, _, _, _| {
            if controller.current_button() == gtk::gdk::BUTTON_PRIMARY {
                messaging::send_message(messaging::ScreensaverMessage::Reset);
            }
        });

        let wrapper = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .vexpand(true)
            .hexpand(true)
            .visible(false)
            .css_classes(["screensaver"])
            .build();

        wrapper.append(&center_wrapper);
        wrapper.add_controller(controller);

        gtk::glib::timeout_add_seconds(1, move || {
            messaging::send_message(ScreensaverMessage::Tick);

            gtk::glib::ControlFlow::Continue
        });

        Self {
            config: config.screensaver.clone(),
            now: Utc::now(),
            last_activity: Utc::now(),
            wrapper,
            center_wrapper,
            date,
            time,
        }
    }

    pub const fn widget(&self) -> &gtk::Box {
        &self.wrapper
    }

    pub fn update(&mut self, message: ScreensaverMessage) {
        match message {
            ScreensaverMessage::Tick => {
                self.now = Utc::now();
                let time = self.now.time();

                if self
                    .config
                    .exclude
                    .iter()
                    .any(|exclude| time >= exclude.start && time <= exclude.end)
                {
                    self.last_activity = self.now;
                }

                if self
                    .config
                    .dim
                    .iter()
                    .any(|dim| time >= dim.start && time <= dim.end)
                {
                    self.center_wrapper.add_css_class("dim");
                } else {
                    self.center_wrapper.remove_css_class("dim");
                }

                self.wrapper.set_visible(
                    (self.now - self.last_activity).num_seconds() > i64::from(self.config.timeout),
                );

                self.time
                    .set_label(&self.now.format("%H:%M:%S").to_string());
                self.date.set_label(
                    &self
                        .now
                        .format_localized("%d. %B", chrono::Locale::de_DE)
                        .to_string(),
                );
            }
            ScreensaverMessage::Reset => {
                self.last_activity = Utc::now();
            }
        }
    }
}
