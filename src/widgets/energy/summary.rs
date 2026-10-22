//! The cards of the energy system on the main screen.

use super::ui::{self, ICON_GRID, ICON_PHOTOVOLTAIC, power};
use crate::enviiq::{Measurement, Overview};
use crate::widgets::summary::{card, detail};

const ICON_CONSUMPTION: char = '\u{f625}'; // gauge-high

/// What is used, produced, stored and bought right now.
pub fn cards(overview: &Overview) -> Vec<gtk::Box> {
    let mut cards = Vec::new();

    if let Some(consumption) = overview.consumption() {
        // The heat pump usually takes the biggest share
        let heat_pump = overview
            .heat_pump
            .as_ref()
            .filter(|heat_pump| heat_pump.is_online)
            .map(|heat_pump| {
                detail(
                    &format!("Wärmepumpe {}", power(heat_pump.consumption())),
                    None,
                )
            });

        cards.push(card(
            ICON_CONSUMPTION,
            "Verbrauch",
            &power(consumption),
            heat_pump,
        ));
    }

    if let Some(photovoltaic) = &overview.photovoltaic {
        cards.push(online(
            photovoltaic,
            ICON_PHOTOVOLTAIC,
            "Photovoltaik",
            || {
                (
                    power(photovoltaic.production()),
                    format!(
                        "Heute {}",
                        ui::energy(photovoltaic.daily_power_production_in_kilo_watt)
                    ),
                    None,
                )
            },
        ));
    }

    if let Some(battery) = &overview.battery {
        let level = battery.battery_level_in_percent;

        cards.push(online(battery, ui::battery_icon(level), "Batterie", || {
            let (state, class) = ui::battery_state(battery);

            (ui::battery_level(level), state, class)
        }));
    }

    if let Some(grid) = &overview.grid {
        cards.push(online(grid, ICON_GRID, "Netz", || {
            let (watts, state, class) = ui::grid_state(grid);

            (power(watts), state.to_string(), class)
        }));
    }

    cards
}

/// A card with the value and detail of the part, offline parts keep their place.
fn online(
    measurement: &Measurement,
    icon: char,
    title: &str,
    content: impl FnOnce() -> (String, String, Option<&'static str>),
) -> gtk::Box {
    if !measurement.is_online {
        return card(
            icon,
            title,
            "–",
            Some(detail("Offline", Some("state-warn"))),
        );
    }

    let (value, text, class) = content();

    card(icon, title, &value, Some(detail(&text, class)))
}
