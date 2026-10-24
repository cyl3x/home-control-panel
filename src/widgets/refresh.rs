//! The refresh button at the end of a page's title row.

use std::time::Duration;

use gtk::glib;

use crate::prelude::*;

/// How long a skipped refresh still spins
const ACKNOWLEDGE: Duration = Duration::from_millis(800);

/// A square button that spins while a refresh it started is running.
#[derive(Clone)]
pub struct RefreshButton {
    button: gtk::Button,
    stack: gtk::Stack,
    spinner: gtk::Spinner,
}

impl RefreshButton {
    pub fn new(on_click: impl Fn() + 'static) -> Self {
        let icon = gtk::Image::from_icon_name("view-refresh-symbolic");

        let spinner = gtk::Spinner::new();
        spinner.add_css_class("refresh__spinner");

        let stack = gtk::Stack::new();
        stack.set_transition_type(gtk::StackTransitionType::Crossfade);
        stack.add_named(&icon, Some("icon"));
        stack.add_named(&spinner, Some("spinner"));

        let button = gtk::Button::new();
        button.set_child(Some(&stack));
        button.add_css_class("refresh");
        button.set_valign(gtk::Align::Center);
        button.update_property(&[gtk::accessible::Property::Label("Aktualisieren")]);
        button.connect_clicked(move |_| on_click());

        Self {
            button,
            stack,
            spinner,
        }
    }

    pub const fn widget(&self) -> &gtk::Button {
        &self.button
    }

    pub fn set_busy(&self, busy: bool) {
        self.spinner.set_spinning(busy);
        self.stack
            .set_visible_child_name(if busy { "spinner" } else { "icon" });
    }

    /// Spin briefly for a skipped refresh, to acknowledge the tap.
    pub fn acknowledge(&self) {
        self.set_busy(true);

        let this = self.clone();
        glib::timeout_add_local_once(ACKNOWLEDGE, move || this.set_busy(false));
    }
}
