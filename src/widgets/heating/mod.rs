use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use chrono::TimeDelta;
use gtk::glib;

use crate::config;
use crate::messaging::{self, HeatingMessage, Page, SwitcherMessage};
use crate::myvaillant::{Action, Command, Error, HeatingSystem, Service, Setpoint};
use crate::prelude::*;
use crate::widgets::refresh::RefreshButton;

mod cards;
mod summary;

/// Refreshes within this time of the last fetch are skipped to protect the API quota
const MIN_REFRESH_INTERVAL: TimeDelta = TimeDelta::seconds(60);

/// Taps on − and + within this many seconds are sent as one change
const SETPOINT_DELAY: u32 = 2;

/// The cloud takes a few seconds to reflect changes, the HA integration waits 5 to 20 seconds
const CHANGE_REFRESH_DELAY: u32 = 10;

/// How long a failed change is noted in its card
const FAILED_NOTE_DURATION: TimeDelta = TimeDelta::seconds(60);

/// Where the note about a change is shown
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Target {
    Zone(String, u32),
    HotWater(String, u32),
    System(String),
}

impl Target {
    fn of_setpoint(setpoint: &Setpoint) -> Self {
        match setpoint {
            Setpoint::Zone { system_id, zone } => Self::Zone(system_id.clone(), *zone),
            Setpoint::HotWater { system_id, index } => Self::HotWater(system_id.clone(), *index),
        }
    }

    fn of_command(command: &Command) -> Self {
        let system_id = command.system_id.clone();

        match command.action {
            Action::ZoneManualSetpoint { zone, .. }
            | Action::ZoneQuickVeto { zone, .. }
            | Action::CancelZoneQuickVeto { zone }
            | Action::ZoneOperationMode { zone, .. } => Self::Zone(system_id, zone),
            Action::HotWaterSetpoint { index, .. }
            | Action::HotWaterOperationMode { index, .. } => Self::HotWater(system_id, index),
            Action::HotWaterBoost { .. }
            | Action::VentilationBoost { .. }
            | Action::AwayMode { .. } => Self::System(system_id),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Note {
    /// Collected or being sent
    Sending,
    /// Sent, the next fetch confirms it
    Sent,
    Failed(DateTime<Local>),
}

pub struct HeatingWidget {
    wrapper: gtk::Overlay,
    status: gtk::Label,
    refresh: RefreshButton,
    /// Status and refresh button, shown next to the name of the first system
    status_bar: gtk::Box,
    systems_box: gtk::Box,
    spinner: gtk::Spinner,
    controls: bool,

    service: Arc<Mutex<Service>>,
    /// The last fetched systems including changes sent since
    systems: Vec<HeatingSystem>,
    /// The last fetched systems, to revert to when a change fails
    confirmed: Vec<HeatingSystem>,
    /// Setpoints changed with − and +, which are not sent yet
    pending: HashMap<Setpoint, (f64, glib::SourceId)>,
    notes: HashMap<Target, Note>,
    fetching: bool,
    refresh_timeout: Option<glib::SourceId>,
    last_fetch: Option<DateTime<Local>>,
    last_success: Option<DateTime<Local>>,
}

impl HeatingWidget {
    pub fn new(config: &config::MyVaillant) -> Self {
        let status = gtk::Label::builder()
            .css_classes(["heating__status"])
            .build();

        let refresh = RefreshButton::new(|| messaging::send_message(HeatingMessage::Refresh));

        let status_bar = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        status_bar.set_halign(gtk::Align::End);
        status_bar.set_hexpand(true);
        status_bar.append(&status);
        status_bar.append(refresh.widget());

        let systems_box = gtk::Box::new(gtk::Orientation::Vertical, 32);
        systems_box.add_css_class("heating__systems");
        // Lets the cards take the height instead of leaving it empty below them
        systems_box.set_vexpand(true);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&systems_box)
            .build();

        let view = gtk::Box::new(gtk::Orientation::Vertical, 0);
        view.set_expand(true);
        view.append(&scrolled);

        let spinner = gtk::Spinner::new();
        spinner.set_expand(true);
        spinner.set_halign(gtk::Align::Center);
        spinner.set_valign(gtk::Align::Center);
        spinner.add_css_class("large");
        spinner.start();

        let wrapper = gtk::Overlay::new();
        wrapper.add_css_class("heating");
        wrapper.add_css_class("spinner-active");
        wrapper.set_expand(true);
        wrapper.set_child(Some(&view));
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

        let mut service = Service::new(config);
        let cached = service.cached();

        // Continue the polling of the last run instead of fetching on every start
        let interval = config.interval.max(60);
        let delay = cached.as_ref().map_or(0, |(_, fetched_at)| {
            let next = *fetched_at + TimeDelta::seconds(interval.into()) - Local::now();

            u32::try_from(next.num_seconds()).unwrap_or(0).min(interval)
        });

        log::info!("Heating: first fetch in {delay} seconds");

        glib::timeout_add_seconds_once(delay, move || {
            messaging::send_message(HeatingMessage::Fetch);

            glib::timeout_add_seconds(interval, || {
                messaging::send_message(HeatingMessage::Fetch);

                glib::ControlFlow::Continue
            });
        });

        let mut widget = Self {
            wrapper,
            status,
            refresh,
            status_bar,
            systems_box,
            spinner,
            controls: config.controls,

            service: Arc::new(Mutex::new(service)),
            systems: Vec::new(),
            confirmed: Vec::new(),
            pending: HashMap::new(),
            notes: HashMap::new(),
            fetching: false,
            refresh_timeout: None,
            last_fetch: None,
            last_success: None,
        };

        if let Some((systems, fetched_at)) = cached {
            log::info!("Heating: showing cached systems from {fetched_at}");

            widget.confirmed = systems.clone();
            widget.systems = systems;
            widget.last_fetch = Some(fetched_at);
            widget.last_success = Some(fetched_at);
            widget
                .status
                .set_text(&format!("Stand {}", cards::format_time(fetched_at)));
            widget.spinner.set_visible(false);
        }

        widget.render();

        widget
    }

    pub const fn widget(&self) -> &gtk::Overlay {
        &self.wrapper
    }

    /// Cards for the main screen, made of the last fetched systems.
    pub fn summary(&self) -> Vec<gtk::Box> {
        summary::cards(&self.systems)
    }

    pub const fn last_success(&self) -> Option<DateTime<Local>> {
        self.last_success
    }

    pub fn update(&mut self, message: HeatingMessage) {
        match message {
            HeatingMessage::Fetch => self.fetch(),
            HeatingMessage::Refresh => {
                if self
                    .last_fetch
                    .is_some_and(|last| Local::now() - last < MIN_REFRESH_INTERVAL)
                {
                    log::info!("Heating: skipping refresh, last fetch is too recent");
                    self.refresh.acknowledge();
                    return;
                }

                self.refresh.set_busy(true);
                self.fetch();
            }
            HeatingMessage::Update(result) => {
                self.fetching = false;
                self.refresh.set_busy(false);
                self.spinner.set_visible(false);

                messaging::send_message(SwitcherMessage::Fetched(Page::Heating, result.is_ok()));

                match *result {
                    Ok(systems) => {
                        log::info!("Heating: fetched {} systems", systems.len());

                        let now = Local::now();
                        self.last_success = Some(now);
                        self.confirmed = systems.clone();
                        self.systems = systems;

                        // The fetch shows whether sent changes are applied
                        self.notes.retain(|_, note| *note != Note::Sent);

                        self.status
                            .set_text(&format!("Aktualisiert {}", cards::format_time(now)));
                        self.status.remove_css_class("error");
                        self.render();
                    }
                    Err(err) => {
                        log::error!("Heating: failed to fetch systems: {err}");

                        self.show_error("Abruf fehlgeschlagen", &err);
                    }
                }
            }
            HeatingMessage::Step(setpoint, steps) => {
                let Some(value) = setpoint.step(&self.displayed(), steps) else {
                    return;
                };

                if let Some((_, timeout)) = self.pending.remove(&setpoint) {
                    remove_source(Some(timeout));
                }

                self.notes
                    .insert(Target::of_setpoint(&setpoint), Note::Sending);

                let key = setpoint.clone();
                let timeout = glib::timeout_add_seconds_local_once(SETPOINT_DELAY, move || {
                    messaging::send_message(HeatingMessage::SendSetpoint(key));
                });

                self.pending.insert(setpoint, (value, timeout));
                self.render();
            }
            HeatingMessage::SendSetpoint(setpoint) => {
                let Some((value, _)) = self.pending.remove(&setpoint) else {
                    return;
                };

                // Decided on the fetched state, e.g. whether a quick veto is already running
                match setpoint.command(&self.systems, value) {
                    Some(command) => self.execute(command),
                    None => {
                        self.notes.remove(&Target::of_setpoint(&setpoint));
                        self.render();
                    }
                }
            }
            HeatingMessage::Execute(command) => self.execute(*command),
            HeatingMessage::Executed(executed) => {
                let (command, result) = *executed;
                let target = Target::of_command(&command);

                match result {
                    Ok(()) => {
                        let tapping = self
                            .pending
                            .keys()
                            .any(|setpoint| Target::of_setpoint(setpoint) == target);

                        if !tapping {
                            self.notes.insert(target, Note::Sent);
                        }
                    }
                    Err(err) => {
                        log::error!("Heating: failed to send change: {err}");

                        self.systems = self.confirmed.clone();
                        self.notes.insert(target, Note::Failed(Local::now()));
                        self.show_error("Änderung fehlgeschlagen", &err);

                        glib::timeout_add_seconds_local_once(
                            FAILED_NOTE_DURATION.num_seconds() as u32,
                            || messaging::send_message(HeatingMessage::ExpireNotes),
                        );
                    }
                }

                self.render();

                // Confirms the change, or reverts the displayed one if it failed
                remove_source(self.refresh_timeout.take());
                self.refresh_timeout = Some(glib::timeout_add_seconds_local_once(
                    CHANGE_REFRESH_DELAY,
                    || messaging::send_message(HeatingMessage::Fetch),
                ));
            }
            HeatingMessage::ExpireNotes => {
                let now = Local::now();

                self.notes.retain(|_, note| match note {
                    Note::Failed(at) => now - *at < FAILED_NOTE_DURATION,
                    _ => true,
                });

                self.render();
            }
        }
    }

    fn fetch(&mut self) {
        if self.fetching {
            log::info!("Heating: skipping fetch, already fetching");
            return;
        }

        log::info!("Heating: fetching systems");

        self.fetching = true;
        self.last_fetch = Some(Local::now());

        self.spinner.set_visible(self.last_success.is_none());

        let service = self.service.clone();

        gtk::gio::spawn_blocking(move || {
            let result = service
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .fetch();

            messaging::send_message(HeatingMessage::Update(std::boxed::Box::new(result)));
        });
    }

    fn execute(&mut self, command: Command) {
        log::info!("Heating: sending {:?}", command.action);

        command.apply(&mut self.systems);
        self.notes
            .insert(Target::of_command(&command), Note::Sending);
        self.render();

        let service = self.service.clone();

        gtk::gio::spawn_blocking(move || {
            let result = service
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .execute(&command);

            messaging::send_message(HeatingMessage::Executed(std::boxed::Box::new((
                command, result,
            ))));
        });
    }

    /// The systems including setpoints that are not sent yet.
    fn displayed(&self) -> Vec<HeatingSystem> {
        let mut systems = self.systems.clone();

        for (setpoint, (value, _)) in &self.pending {
            setpoint.set(&mut systems, *value);
        }

        systems
    }

    fn render(&self) {
        if let Some(parent) = self.status_bar.parent().and_downcast::<gtk::Box>() {
            parent.remove(&self.status_bar);
        }

        while let Some(child) = self.systems_box.first_child() {
            self.systems_box.remove(&child);
        }

        let systems = self.displayed();

        // Without systems the status still has to show up, e.g. for errors
        if systems.is_empty() {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 16);

            if self.last_success.is_some() {
                row.append(&cards::label(
                    "Keine unterstützte Heizung gefunden",
                    "heating__empty",
                ));
            }

            row.append(&self.status_bar);
            self.systems_box.append(&row);
        }

        for (index, system) in systems.iter().enumerate() {
            self.systems_box.append(&cards::system(
                system,
                self.controls,
                (index == 0).then_some(&self.status_bar),
                &self.notes,
            ));
        }
    }

    fn show_error(&self, message: &str, err: &Error) {
        let message = match err {
            Error::QuotaExceeded(until) => format!(
                "API-Limit erreicht, pausiert bis {}",
                cards::format_time(*until)
            ),
            Error::Authentication(_) => "Anmeldung fehlgeschlagen".to_string(),
            _ => message.to_string(),
        };

        // Keep showing the last data, but make clear how old it is
        let message = match self.last_success {
            Some(last_success) => {
                format!("{message} · Stand {}", cards::format_time(last_success))
            }
            None => message,
        };

        self.status.set_text(&message);
        self.status.add_css_class("error");
    }
}
