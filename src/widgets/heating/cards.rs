use crate::messaging::{self, HeatingMessage};
use crate::myvaillant::{Action, Circuit, Command, HeatingSystem, HotWater, Setpoint, Zone};
use crate::prelude::*;

// Font Awesome glyphs, used sparingly where they help finding things at a glance
const ICON_ZONE: char = '\u{f015}'; // house
const ICON_HOT_WATER: char = '\u{f2cc}'; // shower
const ICON_CIRCUIT: char = '\u{f2c9}'; // temperature-half
const ICON_BOOST: char = '\u{f0e7}'; // bolt
const ICON_VENTILATION: char = '\u{f72e}'; // wind
const ICON_AWAY: char = '\u{f5c1}'; // suitcase-rolling

/// A system with its name, overview and cards, the status bar is shown next to the name.
///
/// The overview stats sit in the header row, which leaves the whole height below to the cards.
pub fn system(system: &HeatingSystem, controls: bool, status_bar: Option<&gtk::Box>) -> gtk::Box {
    let overview = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    overview.set_valign(gtk::Align::Center);
    overview.append(&stat(
        "Außen",
        &temperature(system.outdoor_temperature),
        None,
    ));
    overview.append(&stat(
        "Wasserdruck",
        &system.water_pressure.map_or_else(
            || "–".to_string(),
            |pressure| format!("{pressure:.1} bar"),
        ),
        None,
    ));
    let state = system.energy_manager_state.as_deref();
    let state_stat = stat(
        "Status",
        state.map_or("–", describe),
        state.and_then(state_icon),
    );

    if let Some(class) = state.and_then(state_class) {
        state_stat.add_css_class(class);
    }

    overview.append(&state_stat);

    // As many columns as cards fill the width like the rows above, if there is enough space
    let count = system.zones.len() + system.hot_water.len() + system.circuits.len();

    let cards = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .homogeneous(true)
        .min_children_per_line(1)
        .max_children_per_line(u32::try_from(count).unwrap_or(4).clamp(1, 4))
        .row_spacing(16)
        .column_spacing(16)
        // The cards keep their own height. Stretching them to the page only blows up the
        // gap between the reading and the controls.
        .valign(gtk::Align::Start)
        .build();

    for zone in &system.zones {
        cards.append(&zone_card(system, zone, controls));
    }

    for hot_water in &system.hot_water {
        cards.append(&hot_water_card(system, hot_water, controls));
    }

    for circuit in &system.circuits {
        cards.append(&circuit_card(circuit));
    }

    let section = gtk::Box::new(gtk::Orientation::Vertical, 16);
    section.add_css_class("heating__system");
    let title = label(&system.name, "heating__system__title");
    title.set_valign(gtk::Align::Center);

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 32);
    header.add_css_class("heating__header");
    header.append(&title);
    header.append(&overview);

    if let Some(status_bar) = status_bar {
        header.append(status_bar);
    }

    section.append(&header);
    section.append(&cards);

    if controls {
        // Pinned to the bottom, so the height left over sits between the groups rather
        // than inside the cards
        section.append(&grow());
        section.append(&label("Schnellzugriff", "heating__section__title"));
        section.append(&quick_actions(system));
    }

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

pub fn format_time(datetime: DateTime<Local>) -> String {
    if datetime.date_naive() == Local::now().date_naive() {
        datetime.format("%H:%M").to_string()
    } else {
        datetime.format("%d.%m. %H:%M").to_string()
    }
}

/// Toggles for functions of the whole system, like the app's quick access.
fn quick_actions(system: &HeatingSystem) -> gtk::Box {
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    actions.set_homogeneous(true);
    actions.add_css_class("heating__actions");

    for hot_water in &system.hot_water {
        actions.append(&action_button(
            ICON_BOOST,
            "Warmwasser-Boost",
            hot_water.is_boost(),
            system.command(Action::HotWaterBoost {
                index: hot_water.index,
                active: !hot_water.is_boost(),
            }),
        ));
    }

    actions.append(&action_button(
        ICON_VENTILATION,
        "Stoßlüften",
        system.is_ventilation_boost(),
        system.command(Action::VentilationBoost {
            active: !system.is_ventilation_boost(),
        }),
    ));

    actions.append(&action_button(
        ICON_AWAY,
        "Abwesend",
        system.is_away(),
        system.command(Action::AwayMode {
            active: !system.is_away(),
        }),
    ));

    actions
}

fn zone_card(system: &HeatingSystem, zone: &Zone, controls: bool) -> gtk::Box {
    // Systems without a room sensor only know the setpoint
    let (value, setpoint_title) = zone.current_temperature.map_or_else(
        || (zone.desired_temperature, "Solltemperatur"),
        |current| (Some(current), "Soll"),
    );

    let card = card(ICON_ZONE, &zone.name, value, "°C");

    let mut texts = Vec::new();
    texts.extend(
        zone.current_humidity
            .map(|humidity| format!("Luftfeuchte {humidity:.0} %")),
    );
    // With controls the mode buttons show the mode
    texts.extend(
        zone.operation_mode
            .as_deref()
            .filter(|_| !controls)
            .map(|mode| describe(mode).to_string()),
    );

    if let Some(details) = details(&texts, zone.heating_state.as_deref()) {
        card.append(&details);
    }

    card.append(&grow());
    card.append(&setpoint_row(
        setpoint_title,
        &temperature(zone.desired_temperature),
        controls.then(|| Setpoint::Zone {
            system_id: system.id.clone(),
            zone: zone.index,
        }),
    ));

    if controls {
        card.append(&mode_buttons(
            zone.operation_mode.as_deref(),
            system.zone_operation_modes(),
            |mode| {
                system.command(Action::ZoneOperationMode {
                    zone: zone.index,
                    mode: mode.to_string(),
                })
            },
        ));
    }

    if let Some(special_function) = zone.special_function.as_deref() {
        let text = zone.special_function_end.map_or_else(
            || describe(special_function).to_string(),
            |end| {
                format!(
                    "{} bis {}",
                    describe(special_function),
                    format_time(end.with_timezone(&Local))
                )
            },
        );

        let cancel = (controls && zone.is_quick_veto())
            .then(|| system.command(Action::CancelZoneQuickVeto { zone: zone.index }));

        card.append(&highlight(&text, cancel));
    }

    card
}

fn hot_water_card(system: &HeatingSystem, hot_water: &HotWater, controls: bool) -> gtk::Box {
    let card = card(
        ICON_HOT_WATER,
        "Warmwasser",
        hot_water.current_temperature,
        "°C",
    );

    card.append(&grow());
    card.append(&setpoint_row(
        "Soll",
        &temperature(hot_water.setpoint),
        controls.then(|| Setpoint::HotWater {
            system_id: system.id.clone(),
            index: hot_water.index,
        }),
    ));

    if controls {
        card.append(&mode_buttons(
            hot_water.operation_mode.as_deref(),
            system.hot_water_operation_modes(),
            |mode| {
                system.command(Action::HotWaterOperationMode {
                    index: hot_water.index,
                    mode: mode.to_string(),
                })
            },
        ));
    } else if let Some(mode) = hot_water.operation_mode.as_deref() {
        card.append(&label(describe(mode), "heating__card__detail"));
    }

    // A boost is ended with its quick action
    if let Some(special_function) = hot_water.special_function.as_deref() {
        card.append(&highlight(describe(special_function), None));
    }

    card
}

fn circuit_card(circuit: &Circuit) -> gtk::Box {
    let card = card(
        ICON_CIRCUIT,
        &format!("Heizkreis {}", circuit.index + 1),
        circuit.flow_temperature,
        "°C",
    );

    if let Some(details) = details(&["Vorlauf".to_string()], circuit.state.as_deref()) {
        card.append(&details);
    }

    card.append(&grow());

    // The setpoint is 0 while the circuit is in standby, then there is nothing to aim for
    if let Some(setpoint) = circuit.flow_setpoint.filter(|setpoint| *setpoint > 0.0) {
        card.append(&readonly(
            "Vorlauf-Soll",
            &temperature(Some(setpoint)),
        ));
    }

    card
}

/// A card with its title and the big value, the unit stays smaller next to the number.
fn card(icon: char, title: &str, value: Option<f64>, unit: &str) -> gtk::Box {
    let big = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    big.set_margin_top(8);

    match value {
        Some(value) => {
            big.append(&label(&format!("{value:.1}"), "heating__card__value"));

            let unit = label(&format!(" {unit}"), "heating__card__value__unit");
            unit.set_valign(gtk::Align::Baseline);
            big.append(&unit);
        }
        // Without a reading the dash stands alone, a unit would claim there is one
        None => big.append(&label("–", "heating__card__value")),
    }

    let card = gtk::Box::new(gtk::Orientation::Vertical, 12);
    card.add_css_class("heating__card");
    card.append(&with_icon(Some(icon), title, "heating__card__title"));
    card.append(&big);

    card
}

/// Pushes what follows it to the bottom of the card.
fn grow() -> gtk::Box {
    let grow = gtk::Box::new(gtk::Orientation::Vertical, 0);
    grow.set_vexpand(true);
    grow.set_height_request(8);

    grow
}

fn stat(title: &str, value: &str, icon: Option<char>) -> gtk::Box {
    let stat = gtk::Box::new(gtk::Orientation::Vertical, 4);
    stat.add_css_class("heating__stat");
    stat.append(&label(title, "heating__stat__title"));
    stat.append(&with_icon(icon, value, "heating__stat__value"));

    stat
}

/// A label with an icon in front, which takes the size of the label's class.
fn with_icon(icon: Option<char>, text: &str, class: &str) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);

    if let Some(icon) = icon {
        let icon = label(&icon.to_string(), class);
        icon.add_css_class("icon");
        row.append(&icon);
    }

    row.append(&label(text, class));

    row
}

/// The setpoint in a recessed row, with − and + if it can be changed.
fn setpoint_row(title: &str, value: &str, setpoint: Option<Setpoint>) -> gtk::Widget {
    let Some(setpoint) = setpoint else {
        return readonly(title, value).upcast();
    };

    let title = label(title, "heating__setpoint__title");
    title.set_halign(gtk::Align::Center);

    let value = label(value, "heating__setpoint__value");
    value.set_halign(gtk::Align::Center);

    let labels = gtk::Box::new(gtk::Orientation::Vertical, 0);
    labels.set_hexpand(true);
    labels.set_valign(gtk::Align::Center);
    labels.append(&title);
    labels.append(&value);

    let row = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    row.add_css_class("heating__setpoint");
    row.append(&step_button("−", setpoint.clone(), -1));
    row.append(&labels);
    row.append(&step_button("+", setpoint, 1));

    row.upcast()
}

/// A setpoint that cannot be changed here, which reads as a value rather than a control.
fn readonly(title: &str, value: &str) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Vertical, 4);
    row.add_css_class("heating__readonly");
    row.append(&label(title, "heating__readonly__title"));
    row.append(&label(value, "heating__readonly__value"));

    row
}

fn step_button(text: &str, setpoint: Setpoint, steps: i32) -> gtk::Button {
    let button = gtk::Button::with_label(text);
    button.add_css_class("heating__step");
    button.connect_clicked(move |_| {
        messaging::send_message(HeatingMessage::Step(setpoint.clone(), steps));
    });

    button
}

fn mode_buttons(
    current: Option<&str>,
    modes: &[&str],
    command: impl Fn(&str) -> Command,
) -> gtk::Box {
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    buttons.set_homogeneous(true);
    buttons.add_css_class("heating__modes");

    for mode in modes {
        let button = gtk::Button::with_label(mode_label(mode));

        if current == Some(*mode) {
            button.add_css_class("active");
        } else {
            let command = command(mode);
            button.connect_clicked(move |_| execute(&command));
        }

        buttons.append(&button);
    }

    buttons
}

fn action_button(icon: char, text: &str, active: bool, command: Command) -> gtk::Button {
    let content = with_icon(Some(icon), text, "heating__action__label");
    content.set_halign(gtk::Align::Center);

    // These names are short enough to fit, breaking them at the hyphen only looks cramped
    let mut child = content.first_child();
    while let Some(label) = child {
        if let Some(label) = label.downcast_ref::<gtk::Label>() {
            label.set_wrap(false);
        }

        child = label.next_sibling();
    }

    let button = gtk::Button::new();
    button.set_child(Some(&content));
    button.add_css_class("heating__action");
    button.set_class_active("active", active);
    button.connect_clicked(move |_| execute(&command));

    button
}

/// An active special function, optionally with a button to end it.
fn highlight(text: &str, cancel: Option<Command>) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    row.append(&label(text, "heating__card__highlight"));

    if let Some(command) = cancel {
        let button = gtk::Button::with_label("Beenden");
        button.add_css_class("heating__cancel");
        button.connect_clicked(move |_| execute(&command));
        row.append(&button);
    }

    row
}

fn execute(command: &Command) {
    messaging::send_message(HeatingMessage::Execute(std::boxed::Box::new(
        command.clone(),
    )));
}

pub(super) fn temperature(value: Option<f64>) -> String {
    value.map_or_else(|| "–".to_string(), |value| format!("{value:.1} °C"))
}

/// Details in one line, e.g. "Luftfeuchte 70 % · Heizt", with the state colored.
fn details(texts: &[String], state: Option<&str>) -> Option<gtk::Box> {
    let mut labels = texts
        .iter()
        .map(|text| label(text, "heating__card__detail"))
        .collect::<Vec<_>>();

    if let Some(state) = state {
        let state_label = label(describe(state), "heating__card__detail");

        if let Some(class) = state_class(state) {
            state_label.add_css_class(class);
        }

        labels.push(state_label);
    }

    if labels.is_empty() {
        return None;
    }

    let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);

    for (index, label) in labels.iter().enumerate() {
        if index > 0 {
            row.append(&self::label(" · ", "heating__card__detail"));
        }

        // Wrapping single labels of a row would break them apart word by word
        label.set_wrap(false);
        row.append(label);
    }

    Some(row)
}

/// Color class of states worth noticing
pub(super) fn state_class(state: &str) -> Option<&'static str> {
    match state {
        "HEATING" | "HEATING_UP" => Some("state-heating"),
        "COOLING" | "COOLING_DOWN" => Some("state-cooling"),
        _ => None,
    }
}

/// Icon of the energy manager's state
pub(super) fn state_icon(state: &str) -> Option<char> {
    match state {
        "HEATING" => Some('\u{f06d}'), // fire
        "COOLING" => Some('\u{f2dc}'), // snowflake
        "DHW" => Some('\u{f043}'),     // droplet
        "STANDBY" => Some('\u{f04c}'), // pause
        _ => None,
    }
}

/// Short names of operation modes, which have to fit side by side into a card.
fn mode_label(mode: &str) -> &str {
    match mode {
        "TIME_CONTROLLED" | "AUTO" => "Zeitplan",
        "DAY" => "Tag",
        "SET_BACK" => "Absenkung",
        mode => describe(mode),
    }
}

/// Translate the API's states, modes and special functions.
pub(super) fn describe(value: &str) -> &str {
    match value {
        "TIME_CONTROLLED" | "AUTO" => "Zeitprogramm",
        "MANUAL" => "Manuell",
        "DAY" => "Tagbetrieb",
        "SET_BACK" => "Absenkbetrieb",
        "OFF" => "Aus",
        "IDLE" | "STANDBY" => "Standby",
        "HEATING" | "HEATING_UP" => "Heizt",
        "COOLING" | "COOLING_DOWN" => "Kühlt",
        "DHW" => "Warmwasser",
        "QUICK_VETO" => "Quick-Veto",
        "HOLIDAY" => "Abwesend",
        "SYSTEM_OFF" => "System aus",
        "CYLINDER_BOOST" => "Boost",
        "VENTILATION_BOOST" => "Stoßlüften",
        "BANK_HOLIDAY" | "ONE_DAY_BANK_HOLIDAY" => "Feiertag",
        value => value,
    }
}
