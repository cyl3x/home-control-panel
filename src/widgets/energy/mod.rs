use std::sync::{Arc, Mutex, PoisonError};

use chrono::TimeDelta;
use gtk::glib;

use crate::config;
use crate::enviiq::{Error, Overview, Service};
use crate::messaging::{self, EnergyMessage, Page, SwitcherMessage};
use crate::prelude::*;

mod flow;
mod prices;
mod ui;

/// Refreshes within this time of the last fetch are skipped, the box reports about every minute
const MIN_REFRESH_INTERVAL: TimeDelta = TimeDelta::seconds(15);

pub struct EnergyWidget {
    wrapper: gtk::Overlay,
    status: gtk::Label,
    /// Cards and prices, rebuilt on every update
    body: gtk::Box,
    spinner: gtk::Spinner,

    service: Arc<Mutex<Service>>,
    fetching: bool,
    last_fetch: Option<DateTime<Local>>,
    last_success: Option<DateTime<Local>>,
}

impl EnergyWidget {
    pub fn new(config: &config::EnviIq) -> Self {
        let status = gtk::Label::builder()
            .css_classes(["energy__status"])
            .build();

        let refresh_button = gtk::Button::from_icon_name("view-refresh-symbolic");
        refresh_button.set_valign(gtk::Align::Center);
        refresh_button.connect_clicked(|_| {
            messaging::send_message(EnergyMessage::Refresh);
        });

        let title = ui::label("Energie", "energy__title");
        title.set_hexpand(true);

        let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        header.append(&title);
        header.append(&status);
        header.append(&refresh_button);

        let body = gtk::Box::new(gtk::Orientation::Vertical, 16);

        let content = gtk::Box::new(gtk::Orientation::Vertical, 16);
        content.add_css_class("energy__content");
        content.append(&header);
        content.append(&body);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&content)
            .build();

        let spinner = gtk::Spinner::new();
        spinner.set_expand(true);
        spinner.set_halign(gtk::Align::Center);
        spinner.set_valign(gtk::Align::Center);
        spinner.add_css_class("large");
        spinner.start();

        let wrapper = gtk::Overlay::new();
        wrapper.add_css_class("energy");
        wrapper.add_css_class("spinner-active");
        wrapper.set_expand(true);
        wrapper.set_child(Some(&scrolled));
        wrapper.add_overlay(&spinner);

        spinner.connect_visible_notify(glib::clone!(
            #[strong]
            wrapper,
            move |spinner| {
                if spinner.is_visible() {
                    wrapper.add_css_class("spinner-active");
                    spinner.start();
                } else {
                    wrapper.remove_css_class("spinner-active");
                    spinner.stop();
                }
            }
        ));

        messaging::send_message(EnergyMessage::Fetch);

        glib::timeout_add_seconds(config.interval.max(30), || {
            messaging::send_message(EnergyMessage::Fetch);

            glib::ControlFlow::Continue
        });

        Self {
            wrapper,
            status,
            body,
            spinner,

            service: Arc::new(Mutex::new(Service::new(config))),
            fetching: false,
            last_fetch: None,
            last_success: None,
        }
    }

    pub const fn widget(&self) -> &gtk::Overlay {
        &self.wrapper
    }

    pub fn update(&mut self, message: EnergyMessage) {
        match message {
            EnergyMessage::Fetch => self.fetch(),
            EnergyMessage::Refresh => {
                if self
                    .last_fetch
                    .is_some_and(|last| Local::now() - last < MIN_REFRESH_INTERVAL)
                {
                    log::info!("Energy: skipping refresh, last fetch is too recent");
                    return;
                }

                self.fetch();
            }
            EnergyMessage::Update(result) => {
                self.fetching = false;
                self.spinner.set_visible(false);

                messaging::send_message(SwitcherMessage::Fetched(Page::Energy, result.is_ok()));

                match *result {
                    Ok(overview) => {
                        let now = Local::now();
                        self.last_success = Some(now);
                        self.status
                            .set_text(&format!("Aktualisiert {}", ui::format_time(now)));
                        self.status.remove_css_class("error");
                        self.render(&overview, now);
                    }
                    Err(err) => {
                        log::error!("Energy: failed to fetch overview: {err}");

                        self.show_error(&err);
                    }
                }
            }
        }
    }

    fn fetch(&mut self) {
        if self.fetching {
            log::info!("Energy: skipping fetch, already fetching");
            return;
        }

        self.fetching = true;
        self.last_fetch = Some(Local::now());
        self.spinner.set_visible(true);

        let service = self.service.clone();

        gtk::gio::spawn_blocking(move || {
            let result = service
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .fetch();

            messaging::send_message(EnergyMessage::Update(std::boxed::Box::new(result)));
        });
    }

    fn render(&self, overview: &Overview, now: DateTime<Local>) {
        while let Some(child) = self.body.first_child() {
            self.body.remove(&child);
        }

        self.body.append(&flow::diagram(overview));

        if let Some(totals) = &overview.totals {
            self.body.append(&ui::today(totals));
        }

        if let Some(prices) = prices::section(&overview.prices, now) {
            self.body.append(&prices);
        }
    }

    fn show_error(&self, err: &Error) {
        let message = match err {
            Error::Authentication(_) => "Anmeldung fehlgeschlagen",
            _ => "Abruf fehlgeschlagen",
        };

        // Keep showing the last data, but make clear how old it is
        let message = self.last_success.map_or_else(
            || message.to_string(),
            |last_success| format!("{message} · Stand {}", ui::format_time(last_success)),
        );

        self.status.set_text(&message);
        self.status.add_css_class("error");
    }
}
