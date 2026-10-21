use std::collections::HashMap;

use gtk::glib;

use crate::messaging::{Page, SwitcherMessage};
use crate::prelude::*;

/// Failed fetches in a row until a page is marked, single failures are usually just hiccups
const FAILURES_UNTIL_MARKED: u32 = 3;

/// Switches the pages of a stack like `gtk::StackSwitcher`, with a dot before the title
/// of pages that fetch data: green while fetching works, red after repeated failures.
pub struct SwitcherWidget {
    wrapper: gtk::Box,
    dots: HashMap<Page, gtk::Box>,
    failures: HashMap<Page, u32>,
}

impl SwitcherWidget {
    /// Create buttons for the pages of the stack, `pages` maps the names of pages that fetch data.
    pub fn new(stack: &gtk::Stack, pages: &[(Page, &str)]) -> Self {
        let wrapper = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        wrapper.set_homogeneous(true);
        wrapper.set_hexpand(true);
        wrapper.add_css_class("switcher");

        let mut dots = HashMap::new();
        let mut group: Option<gtk::ToggleButton> = None;

        for stack_page in stack.pages().iter::<gtk::StackPage>().flatten() {
            let name = stack_page.name().unwrap_or_default().to_string();
            let title = stack_page.title().unwrap_or_default();

            let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            content.set_halign(gtk::Align::Center);

            if let Some((page, _)) = pages.iter().find(|(_, page_name)| *page_name == name) {
                let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                dot.set_valign(gtk::Align::Center);
                dot.add_css_class("switcher__dot");
                content.append(&dot);
                dots.insert(*page, dot);
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

            wrapper.append(&button);
            group.get_or_insert(button);
        }

        Self {
            wrapper,
            dots,
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
        }
    }
}
