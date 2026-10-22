//! Small cards below the camera on the main screen, a glance at the data of other pages.
//!
//! Each page fills its own row, tapping a row switches to that page.

use std::collections::HashMap;

use chrono::TimeDelta;

use crate::messaging::{self, Page, SwitcherMessage};
use crate::prelude::*;

/// Values of older fetches are grayed out, the dots of the tabs show that fetching fails
const STALE_AFTER: TimeDelta = TimeDelta::minutes(30);

/// Rows the corner below the camera is divided into, also with fewer pages
const ROWS: usize = 2;

pub struct SummaryWidget {
    wrapper: gtk::Box,
    rows: HashMap<Page, gtk::Box>,
}

impl SummaryWidget {
    /// Rows for the pages in the given order, only pages with a configured integration belong here.
    ///
    /// Rows always have the height of a full corner, so the cards look the same with fewer pages.
    /// Missing rows leave space at the top, which keeps the cards next to the calendar's bottom.
    pub fn new(pages: &[Page]) -> Self {
        let wrapper = gtk::Box::new(gtk::Orientation::Vertical, 16);
        wrapper.add_css_class("summary");
        wrapper.set_homogeneous(true);
        wrapper.set_vexpand(true);
        wrapper.set_visible(!pages.is_empty());

        for _ in pages.len()..ROWS {
            wrapper.append(&gtk::Box::new(gtk::Orientation::Horizontal, 0));
        }

        let mut rows = HashMap::new();

        for page in pages.iter().copied() {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 16);
            row.add_css_class("summary__row");
            row.set_homogeneous(true);

            let click = gtk::GestureClick::new();
            click.connect_released(move |_, _, _, _| {
                messaging::send_message(SwitcherMessage::Show(page));
            });
            row.add_controller(click);

            wrapper.append(&row);
            rows.insert(page, row);
        }

        Self { wrapper, rows }
    }

    pub const fn widget(&self) -> &gtk::Box {
        &self.wrapper
    }

    /// Replace the cards of the page, grayed out if the data was fetched too long ago.
    ///
    /// Without cards, e.g. before the first fetch, the row keeps its space empty.
    pub fn set(&self, page: Page, cards: &[gtk::Box], fetched_at: Option<DateTime<Local>>) {
        let Some(row) = self.rows.get(&page) else {
            return;
        };

        while let Some(child) = row.first_child() {
            row.remove(&child);
        }

        for card in cards {
            row.append(card);
        }

        row.set_class_active(
            "stale",
            fetched_at.is_none_or(|fetched_at| Local::now() - fetched_at > STALE_AFTER),
        );
    }
}

/// A card with a label, a large value and an optional line below it.
pub fn card(icon: char, title: &str, value: &str, detail: Option<gtk::Label>) -> gtk::Box {
    let icon = label(&icon.to_string(), "summary__title");
    icon.add_css_class("icon");

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    header.append(&icon);
    let title = label(title, "summary__title");
    title.set_hexpand(true);
    header.append(&title);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.set_vexpand(true);
    content.set_valign(gtk::Align::Center);
    content.append(&header);
    content.append(&label(value, "summary__value"));

    // An empty line keeps the titles and values of a row in line
    content.append(&detail.unwrap_or_else(|| label("", "summary__detail")));

    let card = gtk::Box::new(gtk::Orientation::Vertical, 0);
    card.add_css_class("summary__card");
    card.append(&content);

    card
}

/// The line below the value, the class colors states worth noticing.
pub fn detail(text: &str, class: Option<&str>) -> gtk::Label {
    let detail = label(text, "summary__detail");

    if let Some(class) = class {
        detail.add_css_class(class);
    }

    detail
}

/// Labels are shortened instead of widening the camera's column, which decides the width.
fn label(text: &str, class: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .max_width_chars(8)
        .css_classes([class])
        .build()
}
