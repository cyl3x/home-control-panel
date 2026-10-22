//! Where the power comes from and where it goes, with the house in the middle.
//!
//! Sources are on the left and consumers on the right, so power usually flows from left
//! to right. The battery and the grid can go both ways, the arrows show the direction.

use std::f64::consts::PI;

use gtk::cairo;

use super::ui::{self, detail, power};
use crate::enviiq::{Measurement, Overview};
use crate::prelude::*;

// Font Awesome glyphs, used sparingly where they help finding things at a glance
const ICON_PHOTOVOLTAIC: char = '\u{f5ba}'; // solar-panel
const ICON_GRID: char = '\u{f1e6}'; // plug
const ICON_HOUSE: char = '\u{f015}'; // house
const ICON_HOUSEHOLD: char = '\u{f0eb}'; // lightbulb
const ICON_HEAT_PUMP: char = '\u{f863}'; // fan

const NODE_WIDTH: i32 = 200;
/// Diameter of the house in the middle, the border radius in the CSS matches it
const HUB_SIZE: i32 = 150;
/// Minimal length of the lines between the parts and the house
const LINE_SPACE: i32 = 48;
/// Distance between the lines where they meet the house
const LINE_SPREAD: f64 = 16.0;
const ARROW_SIZE: f64 = 7.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flow {
    Idle,
    ToHouse,
    FromHouse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

struct Line {
    node: gtk::Box,
    side: Side,
    flow: Flow,
    /// Position among the lines of the same side, to spread them where they meet the house
    index: usize,
    count: usize,
}

pub fn diagram(overview: &Overview) -> gtk::Overlay {
    let mut lines = Vec::new();

    let sources = column(
        [
            overview.photovoltaic.as_ref().map(photovoltaic),
            overview.battery.as_ref().map(battery),
            overview.grid.as_ref().map(grid),
        ],
        Side::Left,
        &mut lines,
    );

    let consumers = column(
        [
            overview
                .house
                .as_ref()
                .map(|house| consumer(ICON_HOUSEHOLD, "Haushalt", house, None)),
            overview.heat_pump.as_ref().map(heat_pump),
        ],
        Side::Right,
        &mut lines,
    );

    let hub = hub(overview);

    let layout = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    layout.append(&sources);
    layout.append(&spacer());
    layout.append(&hub);
    layout.append(&spacer());
    layout.append(&consumers);

    let area = gtk::DrawingArea::new();
    area.add_css_class("energy__flow");
    area.set_draw_func(move |area, cr, _, _| {
        if let Err(err) = draw(area, cr, &hub, &lines) {
            log::warn!("Energy: failed to draw the flow: {err}");
        }
    });

    // The lines are drawn below the parts, which decide the size
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&area));
    overlay.add_overlay(&layout);
    overlay.set_measure_overlay(&layout, true);

    overlay
}

/// The parts of one side, spread evenly over the height of the diagram.
fn column<const N: usize>(
    nodes: [Option<(gtk::Box, Flow)>; N],
    side: Side,
    lines: &mut Vec<Line>,
) -> gtk::Box {
    let nodes = nodes.into_iter().flatten().collect::<Vec<_>>();
    let count = nodes.len();

    let column = gtk::Box::new(gtk::Orientation::Vertical, 8);
    column.set_homogeneous(true);

    for (index, (node, flow)) in nodes.into_iter().enumerate() {
        column.append(&node);
        lines.push(Line {
            node,
            side,
            flow,
            index,
            count,
        });
    }

    column
}

fn spacer() -> gtk::Box {
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    spacer.set_width_request(LINE_SPACE);

    spacer
}

/// The house with the power used by all consumers.
fn hub(overview: &Overview) -> gtk::Box {
    let total = [&overview.house, &overview.heat_pump]
        .into_iter()
        .flatten()
        .filter(|consumer| consumer.is_online)
        .map(Measurement::consumption)
        .reduce(|a, b| a + b);

    let icon = ui::label(&ICON_HOUSE.to_string(), "energy__hub__icon");
    icon.add_css_class("icon");

    let value = ui::label(
        &total.map_or_else(|| "–".to_string(), power),
        "energy__hub__value",
    );

    let content = gtk::Box::new(gtk::Orientation::Vertical, 2);
    content.set_vexpand(true);
    content.set_valign(gtk::Align::Center);

    for label in [&icon, &value, &detail("Verbrauch", None)] {
        label.set_halign(gtk::Align::Center);
        content.append(label);
    }

    let hub = gtk::Box::new(gtk::Orientation::Vertical, 0);
    hub.add_css_class("energy__hub");
    hub.set_size_request(HUB_SIZE, HUB_SIZE);
    hub.set_valign(gtk::Align::Center);
    // Centers the content in the circle without stretching the page
    hub.set_vexpand(false);
    hub.append(&content);

    hub
}

fn photovoltaic(photovoltaic: &Measurement) -> (gtk::Box, Flow) {
    if !photovoltaic.is_online {
        return offline(ICON_PHOTOVOLTAIC, "Photovoltaik");
    }

    let flow = if photovoltaic.is_producing {
        Flow::ToHouse
    } else {
        Flow::Idle
    };

    (
        node(
            ICON_PHOTOVOLTAIC,
            "Photovoltaik",
            &power(photovoltaic.production()),
            None,
        ),
        flow,
    )
}

fn battery(battery: &Measurement) -> (gtk::Box, Flow) {
    let level = battery.battery_level_in_percent;
    let icon = battery_icon(level);

    if !battery.is_online {
        return offline(icon, "Batterie");
    }

    // The battery consumes while charging
    let (state, flow) = if battery.is_consuming {
        (
            detail(
                &format!("Lädt {}", power(battery.consumption())),
                Some("state-good"),
            ),
            Flow::FromHouse,
        )
    } else if battery.is_producing {
        (
            detail(&format!("Entlädt {}", power(battery.production())), None),
            Flow::ToHouse,
        )
    } else {
        (detail("Ruht", None), Flow::Idle)
    };

    (
        node(
            icon,
            "Batterie",
            &level.map_or_else(|| "–".to_string(), |level| format!("{level:.0} %")),
            Some(state),
        ),
        flow,
    )
}

fn grid(grid: &Measurement) -> (gtk::Box, Flow) {
    if !grid.is_online {
        return offline(ICON_GRID, "Netz");
    }

    // The grid produces while power is bought
    let (watts, state, flow) = if grid.is_producing {
        (
            grid.production(),
            detail("Bezug", Some("state-warn")),
            Flow::ToHouse,
        )
    } else if grid.is_consuming {
        (
            grid.consumption(),
            detail("Einspeisung", Some("state-good")),
            Flow::FromHouse,
        )
    } else {
        (0.0, detail("Ausgeglichen", None), Flow::Idle)
    };

    (node(ICON_GRID, "Netz", &power(watts), Some(state)), flow)
}

fn heat_pump(heat_pump: &Measurement) -> (gtk::Box, Flow) {
    let state = heat_pump.command.as_deref().map(|command| {
        let (text, class) = sg_ready_state(command);
        detail(text, class)
    });

    consumer(ICON_HEAT_PUMP, "Wärmepumpe", heat_pump, state)
}

fn consumer(
    icon: char,
    title: &str,
    consumer: &Measurement,
    state: Option<gtk::Label>,
) -> (gtk::Box, Flow) {
    if !consumer.is_online {
        return offline(icon, title);
    }

    let flow = if consumer.is_consuming {
        Flow::FromHouse
    } else {
        Flow::Idle
    };

    (
        node(icon, title, &power(consumer.consumption()), state),
        flow,
    )
}

fn offline(icon: char, title: &str) -> (gtk::Box, Flow) {
    (
        node(
            icon,
            title,
            "–",
            Some(detail("Offline", Some("state-warn"))),
        ),
        Flow::Idle,
    )
}

fn node(icon: char, title: &str, value: &str, state: Option<gtk::Label>) -> gtk::Box {
    let node = gtk::Box::new(gtk::Orientation::Vertical, 0);
    node.add_css_class("energy__node");
    node.set_width_request(NODE_WIDTH);
    node.set_valign(gtk::Align::Center);
    node.append(&ui::with_icon(icon, title, "energy__node__title"));
    node.append(&ui::label(value, "energy__node__value"));

    if let Some(state) = state {
        node.append(&state);
    }

    node
}

fn draw(
    area: &gtk::DrawingArea,
    cr: &cairo::Context,
    hub: &gtk::Box,
    lines: &[Line],
) -> Result<(), cairo::Error> {
    let Some(hub) = hub.compute_bounds(area) else {
        return Ok(());
    };

    let center_x = f64::from(hub.x() + hub.width() / 2.0);
    let center_y = f64::from(hub.y() + hub.height() / 2.0);
    let radius = f64::from(hub.width() / 2.0);
    let color = area.color();

    for line in lines {
        let Some(node) = line.node.compute_bounds(area) else {
            continue;
        };

        let node_y = f64::from(node.y() + node.height() / 2.0);
        let offset = (line.index as f64 - (line.count - 1) as f64 / 2.0) * LINE_SPREAD;

        // Where the line meets the circle of the house
        let hub_x = ((radius - offset) * (radius + offset)).max(0.0).sqrt();
        let hub_y = center_y + offset;

        let (start, end) = match line.side {
            Side::Left => (
                (f64::from(node.x() + node.width()), node_y),
                (center_x - hub_x, hub_y),
            ),
            Side::Right => ((center_x + hub_x, hub_y), (f64::from(node.x()), node_y)),
        };

        let forward = matches!(
            (line.side, line.flow),
            (Side::Left, Flow::ToHouse) | (Side::Right, Flow::FromHouse)
        );

        curve(cr, start, end, line.flow, forward, &color)?;
    }

    Ok(())
}

/// A horizontal S-curve, active ones with an arrow in the middle pointing in the direction of
/// the power.
fn curve(
    cr: &cairo::Context,
    start: (f64, f64),
    end: (f64, f64),
    flow: Flow,
    forward: bool,
    color: &gtk::gdk::RGBA,
) -> Result<(), cairo::Error> {
    let middle = f64::midpoint(start.0, end.0);
    let control_start = (middle, start.1);
    let control_end = (middle, end.1);

    cr.move_to(start.0, start.1);
    cr.curve_to(
        control_start.0,
        control_start.1,
        control_end.0,
        control_end.1,
        end.0,
        end.1,
    );

    let (red, green, blue) = (
        f64::from(color.red()),
        f64::from(color.green()),
        f64::from(color.blue()),
    );

    if flow == Flow::Idle {
        cr.set_source_rgba(red, green, blue, 0.25);
        cr.set_line_width(2.0);

        return cr.stroke();
    }

    cr.set_source_rgb(red, green, blue);
    cr.set_line_width(3.0);
    cr.stroke()?;

    // The curve is point symmetric, so its middle is halfway between the ends,
    // where the slope of the Bézier curve is twice the one of the straight line
    let x = f64::midpoint(start.0, end.0);
    let y = f64::midpoint(start.1, end.1);
    let angle = (2.0 * (end.1 - start.1)).atan2(end.0 - start.0) + if forward { 0.0 } else { PI };

    cr.save()?;
    cr.translate(x, y);
    cr.rotate(angle);
    cr.move_to(ARROW_SIZE, 0.0);
    cr.line_to(-ARROW_SIZE, -ARROW_SIZE);
    cr.line_to(-ARROW_SIZE, ARROW_SIZE);
    cr.close_path();
    cr.fill()?;
    cr.restore()
}

/// SG Ready states of the heat pump, named like the app does.
fn sg_ready_state(command: &str) -> (&str, Option<&'static str>) {
    match command {
        "normal" => ("Freier Betrieb", None),
        "blocked" => ("Zwangsabschaltung", Some("state-warn")),
        "recommended-on" => ("Empfohlen ein", Some("state-good")),
        "forced-on" => ("Erzwungen ein", Some("state-good")),
        command => (command, None),
    }
}

/// Battery glyph filled like the battery, from full (f240) to empty (f244).
fn battery_icon(level: Option<f64>) -> char {
    match level.unwrap_or_default() {
        level if level >= 88.0 => '\u{f240}',
        level if level >= 63.0 => '\u{f241}',
        level if level >= 38.0 => '\u{f242}',
        level if level >= 13.0 => '\u{f243}',
        _ => '\u{f244}',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_battery_icon() {
        assert_eq!(battery_icon(Some(100.0)), '\u{f240}');
        assert_eq!(battery_icon(Some(70.0)), '\u{f241}');
        assert_eq!(battery_icon(Some(5.0)), '\u{f244}');
        assert_eq!(battery_icon(None), '\u{f244}');
    }
}
