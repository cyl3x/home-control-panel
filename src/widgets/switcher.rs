use std::collections::HashMap;

use gtk::glib;

use crate::messaging::{Page, SwitcherMessage};
use crate::prelude::*;

/// Failed fetches in a row until a page is marked, single failures are usually just hiccups
const FAILURES_UNTIL_MARKED: u32 = 3;

/// Switches the pages of a stack like `gtk::StackSwitcher`, with a dot before the title
/// of pages that fetch data: green while fetching works, red after repeated failures.
///
/// The buttons are pills in a recessed track, the date and time sit at the end of the bar.
pub struct SwitcherWidget {
    wrapper: gtk::Box,
    dots: HashMap<Page, gtk::Box>,
    buttons: HashMap<Page, gtk::ToggleButton>,
    failures: HashMap<Page, u32>,
}

impl SwitcherWidget {
    /// Create buttons for the pages of the stack, `pages` maps the names of pages that fetch data.
    pub fn new(stack: &gtk::Stack, pages: &[(Page, &str)]) -> Self {
        let track = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        track.set_halign(gtk::Align::Start);
        track.add_css_class("switcher__track");

        let wrapper = gtk::Box::new(gtk::Orientation::Horizontal, 24);
        wrapper.set_hexpand(true);
        wrapper.add_css_class("switcher");
        wrapper.append(&track);
        wrapper.append(&clock());

        let mut dots = HashMap::new();
        let mut buttons = HashMap::new();
        let mut group: Option<gtk::ToggleButton> = None;

        for stack_page in stack.pages().iter::<gtk::StackPage>().flatten() {
            let name = stack_page.name().unwrap_or_default().to_string();
            let title = stack_page.title().unwrap_or_default();

            let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            content.set_halign(gtk::Align::Center);

            let page = pages
                .iter()
                .find(|(_, page_name)| *page_name == name)
                .map(|(page, _)| *page);

            if let Some(page) = page {
                let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                dot.set_valign(gtk::Align::Center);
                dot.add_css_class("switcher__dot");
                content.append(&dot);
                dots.insert(page, dot);
            }

            content.append(&gtk::Label::new(Some(&title)));

            let button = gtk::ToggleButton::new();
            button.set_child(Some(&content));
            button.set_group(group.as_ref());
            button.set_active(stack.visible_child_name().as_deref() == Some(name.as_str()));
            button.connect_toggled(glib::clone!(
                #[strong]
                stack,
                move |button| {
                    if button.is_active() {
                        stack.set_visible_child_name(&name);
                    }
                }
            ));

            track.append(&button);

            if let Some(page) = page {
                buttons.insert(page, button.clone());
            }

            group.get_or_insert(button);
        }

        Self {
            wrapper,
            dots,
            buttons,
            failures: HashMap::new(),
        }
    }

    pub const fn widget(&self) -> &gtk::Box {
        &self.wrapper
    }

    pub fn update(&mut self, message: SwitcherMessage) {
        match message {
            SwitcherMessage::Fetched(page, success) => {
                let failures = self.failures.entry(page).or_default();
                *failures = if success { 0 } else { *failures + 1 };

                let failing = *failures >= FAILURES_UNTIL_MARKED;

                if *failures == FAILURES_UNTIL_MARKED {
                    log::warn!("Switcher: {page:?} failed to fetch {failures} times in a row");
                }

                if let Some(dot) = self.dots.get(&page) {
                    dot.set_class_active("failing", failing);
                }
            }
            SwitcherMessage::Show(page) => {
                if let Some(button) = self.buttons.get(&page) {
                    button.set_active(true);
                }
            }
        }
    }
}

/// Date and time at the end of the bar, so the panel answers that without waking the screensaver.
///
/// Display only, so it keeps its own ticker instead of going through the app's messages.
fn clock() -> gtk::Box {
    let date = gtk::Label::builder().css_classes(["clock__date"]).build();
    let time = gtk::Label::builder().css_classes(["clock__time"]).build();

    let wrapper = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    wrapper.set_hexpand(true);
    wrapper.set_halign(gtk::Align::End);
    wrapper.set_valign(gtk::Align::Center);
    wrapper.add_css_class("clock");
    wrapper.append(&date);
    wrapper.append(&time);

    let tick = move || {
        let now = Local::now();

        date.set_label(
            &now.format_localized("%A, %-d. %B", chrono::Locale::de_DE)
                .to_string(),
        );
        time.set_label(&now.format("%H:%M").to_string());
    };

    tick();

    // Seconds are not shown, but ticking every second keeps the minute from lagging behind
    glib::timeout_add_seconds_local(1, move || {
        tick();

        glib::ControlFlow::Continue
    });

    wrapper
}
