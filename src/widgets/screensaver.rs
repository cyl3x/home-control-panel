use std::time::Duration;

use chrono::DateTime;
use chrono::Local;
use gtk::glib::Priority;

use crate::config;
use crate::config::Config;
use crate::messaging;
use crate::messaging::ScreensaverMessage;
use crate::prelude::*;
use crate::widgets::calendar::birthday::BirthdayLine;
use crate::widgets::calendar::upcoming::UpcomingWidget;

pub struct ScreensaverWidget {
    now: DateTime<Local>,
    last_activity: DateTime<Local>,
    config: config::Screensaver,

    wrapper: gtk::Box,
    center_wrapper: gtk::Box,
    date: gtk::Label,
    time: gtk::Label,
    seconds: gtk::Label,
}

impl ScreensaverWidget {
    pub fn new(config: &Config, upcoming: &UpcomingWidget, birthday: &BirthdayLine) -> Self {
        let date = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .css_classes(["screensaver__date"])
            .build();

        let time = gtk::Label::builder()
            .css_classes(["screensaver__time"])
            .build();

        // The seconds are smaller and dimmer, so the minute stays the thing you read
        let seconds = gtk::Label::builder()
            .valign(gtk::Align::Baseline)
            .css_classes(["screensaver__seconds"])
            .build();

        let time_row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(0)
            .halign(gtk::Align::Start)
            .build();

        time_row.append(&time);
        time_row.append(&seconds);

        let clock = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(16)
            .halign(gtk::Align::Start)
            .valign(gtk::Align::Center)
            .build();

        clock.append(&time_row);
        clock.append(&date);
        clock.append(birthday.widget());

        let rule = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .valign(gtk::Align::Center)
            .css_classes(["screensaver__rule"])
            .build();

        rule.set_size_request(1, 280);

        // Time and agenda side by side, which keeps the whole thing compact and centered
        let center_wrapper = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(64)
            .hexpand(false)
            .vexpand(true)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .css_classes(["screensaver__center"])
            .build();

        center_wrapper.append(&clock);
        center_wrapper.append(&rule);
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

        gtk::glib::timeout_add_local_full(Duration::from_secs(1), Priority::DEFAULT_IDLE, move || {
            messaging::send_message(ScreensaverMessage::Tick);

            gtk::glib::ControlFlow::Continue
        });

        Self {
            config: config.screensaver.clone(),
            now: Local::now(),
            last_activity: Local::now(),
            wrapper,
            center_wrapper,
            date,
            time,
            seconds,
        }
    }

    pub const fn widget(&self) -> &gtk::Box {
        &self.wrapper
    }

    pub fn update(&mut self, message: ScreensaverMessage) {
        match message {
            ScreensaverMessage::Tick => {
                self.now = Local::now();
                let time = self.now.time();

                if self
                    .config
                    .exclude
                    .iter()
                    .any(|exclude| time >= exclude.start && time <= exclude.end)
                {
                    self.last_activity = self.now;
                }

                let dim = self
                    .config
                    .dim
                    .iter()
                    .any(|dim| time >= dim.start && time <= dim.end);

                self.center_wrapper.set_class_active("dim", dim);

                // The panel goes dark at the same times
                if let Some(root) = self.wrapper.root() {
                    root.set_class_active("night", dim);
                }


                let activate = (self.now - self.last_activity).num_seconds() > i64::from(self.config.timeout);

                if activate != self.wrapper.is_visible() {
                    log::info!("Screensaver: {}", if activate { "activated" } else { "deactivated" });
                }

                self.wrapper.set_visible(activate);

                self.time.set_label(&self.now.format("%H:%M").to_string());
                self.seconds.set_label(&self.now.format(":%S").to_string());
                self.date.set_label(
                    &self
                        .now
                        .format_localized("%A, %-d. %B", chrono::Locale::de_DE)
                        .to_string(),
                );
            }
            ScreensaverMessage::Reset => {
                log::debug!("Screensaver: reset due to user activity");

                self.last_activity = Local::now();
            }
        }
    }
}
