//! The cards of the heating on the main screen.

use super::cards::{describe, state_class, state_icon, temperature};
use crate::myvaillant::HeatingSystem;
use crate::prelude::*;
use crate::widgets::summary::{card, detail};

// Font Awesome glyphs, used sparingly where they help finding things at a glance
const ICON_INSIDE: char = '\u{f015}'; // house
const ICON_HOT_WATER: char = '\u{f2cc}'; // shower
const ICON_OUTSIDE: char = '\u{f6c4}'; // cloud-sun
const ICON_STATE: char = '\u{f06d}'; // fire

/// Temperatures inside, of the hot water and outside, and what the heating does right now.
///
/// Only the first zone of the first system is shown, which is all most homes have.
pub fn cards(systems: &[HeatingSystem]) -> Vec<gtk::Box> {
    let Some(system) = systems.first() else {
        return Vec::new();
    };

    let zone = system.zones.first();
    let mut cards = Vec::new();

    if let Some(zone) = zone {
        cards.push(card(
            ICON_INSIDE,
            "Innen",
            &temperature(zone.current_temperature),
            Some(setpoint(zone.desired_temperature)),
        ));
    }

    if let Some(hot_water) = system.hot_water.first() {
        cards.push(card(
            ICON_HOT_WATER,
            "Warmwasser",
            &temperature(hot_water.current_temperature),
            Some(setpoint(hot_water.setpoint)),
        ));
    }

    cards.push(card(
        ICON_OUTSIDE,
        "Außen",
        &temperature(system.outdoor_temperature),
        None,
    ));

    if let Some(state) = system.energy_manager_state.as_deref() {
        // A special function like a quick veto matters more than the usual mode
        let mode = zone
            .and_then(|zone| {
                zone.special_function
                    .as_deref()
                    .or(zone.operation_mode.as_deref())
            })
            .map(|mode| detail(describe(mode), None));

        let card = card(
            state_icon(state).unwrap_or(ICON_STATE),
            "Heizung",
            describe(state),
            mode,
        );

        if let Some(class) = state_class(state) {
            card.add_css_class(class);
        }

        cards.push(card);
    }

    cards
}

fn setpoint(value: Option<f64>) -> gtk::Label {
    detail(&format!("Soll {}", temperature(value)), None)
}
