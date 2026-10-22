use crate::enviiq::{Measurement, Totals};
use crate::prelude::*;

// Font Awesome glyphs, used sparingly where they help finding things at a glance
pub const ICON_PHOTOVOLTAIC: char = '\u{f5ba}'; // solar-panel
pub const ICON_GRID: char = '\u{f1e6}'; // plug
pub const ICON_HOUSE: char = '\u{f015}'; // house
pub const ICON_HOUSEHOLD: char = '\u{f0eb}'; // lightbulb
pub const ICON_HEAT_PUMP: char = '\u{f863}'; // fan

/// The totals of today, as a block of four that sits beside the flow diagram.
pub fn today(totals: &Totals) -> gtk::Box {
    let stats = [
        stat("Erzeugt", &energy(totals.production)),
        stat("Verbraucht", &energy(totals.consumption)),
        stat(
            "Solaranteil",
            &totals
                .share_of_total_power_consumption_in_percent
                .map_or_else(|| "–".to_string(), |share| format!("{share:.0} %")),
        ),
        stat("Netzbezug", &energy(totals.grid_consumption)),
    ];

    let grid = gtk::Grid::builder()
        .row_spacing(16)
        .column_spacing(16)
        .row_homogeneous(true)
        .column_homogeneous(true)
        .vexpand(true)
        .hexpand(true)
        .build();

    for (index, stat) in stats.into_iter().enumerate() {
        grid.attach(&stat, (index % 2) as i32, (index / 2) as i32, 1, 1);
    }

    let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    section.set_hexpand(false);
    section.set_width_request(360);
    section.append(&label("Heute", "energy__section__title"));
    section.append(&grid);

    section
}

pub fn label(text: &str, class: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .halign(gtk::Align::Start)
        .wrap(true)
        .css_classes([class])
        .build()
}

/// A label with an icon in front, which takes the size of the label's class.
pub fn with_icon(icon: char, text: &str, class: &str) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);

    let icon = label(&icon.to_string(), class);
    icon.add_css_class("icon");
    row.append(&icon);
    row.append(&label(text, class));

    row
}

/// A smaller, gray line of text, the class colors states worth noticing.
pub fn detail(text: &str, class: Option<&str>) -> gtk::Label {
    let detail = label(text, "energy__detail");

    if let Some(class) = class {
        detail.add_css_class(class);
    }

    detail
}

pub fn format_time(datetime: DateTime<Local>) -> String {
    if datetime.date_naive() == Local::now().date_naive() {
        datetime.format("%H:%M").to_string()
    } else {
        datetime.format("%d.%m. %H:%M").to_string()
    }
}

/// What the battery does and the class to color it, the battery consumes while charging.
pub fn battery_state(battery: &Measurement) -> (String, Option<&'static str>) {
    if battery.is_producing() {
        (format!("Entlädt {}", power(battery.production())), None)
    } else if battery.is_consuming() {
        (
            format!("Lädt {}", power(battery.consumption())),
            Some("state-good"),
        )
    } else {
        ("Ruht".to_string(), None)
    }
}

/// The power at the grid, what happens there and the class to color it, the grid produces
/// while power is bought.
pub fn grid_state(grid: &Measurement) -> (f64, &'static str, Option<&'static str>) {
    if grid.is_producing() {
        (grid.production(), "Bezug", Some("state-warn"))
    } else if grid.is_consuming() {
        (grid.consumption(), "Einspeisung", Some("state-good"))
    } else {
        (0.0, "Ausgeglichen", None)
    }
}

/// SG Ready states of the heat pump and the class to color them, named like the app does.
pub fn sg_ready_state(command: &str) -> (&str, Option<&'static str>) {
    match command {
        "normal" => ("Freier Betrieb", None),
        "blocked" => ("Zwangsabschaltung", Some("state-warn")),
        "recommended-on" => ("Empfohlen ein", Some("state-good")),
        "forced-on" => ("Erzwungen ein", Some("state-good")),
        command => (command, None),
    }
}

pub fn battery_level(level: Option<f64>) -> String {
    level.map_or_else(|| "–".to_string(), |level| format!("{level:.0} %"))
}

/// Battery glyph filled like the battery, from full (f240) to empty (f244).
pub fn battery_icon(level: Option<f64>) -> char {
    match level.unwrap_or_default() {
        level if level >= 88.0 => '\u{f240}',
        level if level >= 63.0 => '\u{f241}',
        level if level >= 38.0 => '\u{f242}',
        level if level >= 13.0 => '\u{f243}',
        _ => '\u{f244}',
    }
}

pub fn power(watts: f64) -> String {
    if watts.abs() < 1000.0 {
        format!("{watts:.0} W")
    } else {
        format!("{:.2} kW", watts / 1000.0)
    }
}

pub fn energy(kwh: Option<f64>) -> String {
    kwh.map_or_else(|| "–".to_string(), |kwh| format!("{kwh:.1} kWh"))
}

fn stat(title: &str, value: &str) -> gtk::Box {
    // The title sits at the top and the value at the bottom, so the values line up
    let grow = gtk::Box::new(gtk::Orientation::Vertical, 0);
    grow.set_vexpand(true);

    let stat = gtk::Box::new(gtk::Orientation::Vertical, 4);
    stat.add_css_class("energy__stat");
    stat.append(&label(title, "energy__stat__title"));
    stat.append(&grow);
    stat.append(&label(value, "energy__stat__value"));

    stat
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_power() {
        assert_eq!(power(500.0), "500 W");
        assert_eq!(power(999.4), "999 W");
        assert_eq!(power(2500.0), "2.50 kW");
    }

    #[test]
    fn picks_battery_icon() {
        assert_eq!(battery_icon(Some(100.0)), '\u{f240}');
        assert_eq!(battery_icon(Some(70.0)), '\u{f241}');
        assert_eq!(battery_icon(Some(5.0)), '\u{f244}');
        assert_eq!(battery_icon(None), '\u{f244}');
    }
}
