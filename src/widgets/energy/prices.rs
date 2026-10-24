use super::ui::{cent, label};
use crate::enviiq::Price;
use crate::prelude::*;
use crate::widgets::format::decimal;

/// Smallest height the chart is useful at, above that it grows with the page
const CHART_MIN_HEIGHT: i32 = 160;

/// Rows per segment of the scale, the bars are measured in rows
const SEGMENT_ROWS: i32 = 12;

/// Most segments of the scale
const MAX_SEGMENTS: i32 = 5;

/// Width of the ct scale left of the bars
const SCALE_WIDTH: i32 = 64;

const BAR_SPACING: i32 = 6;

/// The exchange prices of the day as bars, with the current and the cheapest remaining slot marked.
pub fn section(prices: &[Price], now: DateTime<Local>) -> Option<gtk::Box> {
    let min = prices.iter().map(|price| price.price).reduce(f64::min)?;
    let max = prices.iter().map(|price| price.price).reduce(f64::max)?;

    let current = prices
        .iter()
        .position(|price| price.from <= now && now < price.to);

    // The first of equally cheap slots is the earliest
    let cheapest = current.and_then(|current| {
        prices[current..]
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.price.total_cmp(&b.price))
            .map(|(offset, _)| current + offset)
    });

    let title = label("Börsenpreis heute", "energy__section__title");
    title.set_hexpand(true);
    title.set_valign(gtk::Align::Start);

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 40);
    header.append(&title);

    match (current, cheapest) {
        (Some(current), Some(cheapest)) if cheapest == current => {
            header.append(&key(
                Some("current"),
                "Jetzt am günstigsten",
                &cent(prices[current].price),
            ));
        }
        (Some(current), Some(cheapest)) => {
            header.append(&key(Some("current"), "Jetzt", &cent(prices[current].price)));
            header.append(&key(
                Some("cheapest"),
                "Am günstigsten",
                &format!(
                    "{} · {}",
                    prices[cheapest].from.format("%H:%M"),
                    cent(prices[cheapest].price)
                ),
            ));
        }
        _ => {
            header.append(&key(
                None,
                "Spanne",
                &format!("{} – {}", decimal(min, 1), cent(max)),
            ));
        }
    }

    let (step, above, below) = scale(min, max);
    let rows = SEGMENT_ROWS * (above + below);
    let zero = SEGMENT_ROWS * above;
    let digits = usize::from(step < 1.0);

    let bars = gtk::Grid::builder()
        .row_homogeneous(true)
        .column_homogeneous(true)
        .column_spacing(BAR_SPACING)
        .height_request(CHART_MIN_HEIGHT)
        .margin_start(SCALE_WIDTH)
        .vexpand(true)
        .hexpand(true)
        .build();

    // Spans every row, so rows no bar reaches aren't collapsed
    bars.attach(&gtk::Box::new(gtk::Orientation::Vertical, 0), 0, 0, 1, rows);

    let hours = gtk::Box::new(gtk::Orientation::Horizontal, BAR_SPACING);
    hours.set_homogeneous(true);

    for (index, price) in prices.iter().enumerate() {
        let negative = price.price < 0.0;
        let height = ((price.price.abs() / step * f64::from(SEGMENT_ROWS)).round() as i32).max(1);

        let (row, height, free) = if negative {
            let height = height.min(rows - zero);
            (zero, height, zero)
        } else {
            let height = height.min(zero);
            (zero - height, height, zero - height)
        };

        let bar = gtk::Box::new(gtk::Orientation::Vertical, 0);
        bar.add_css_class("energy__bar");
        bar.set_valign(gtk::Align::Fill);

        if Some(index) == current {
            bar.add_css_class("current");
        } else if Some(index) == cheapest {
            bar.add_css_class("cheapest");
        } else if price.to <= now {
            bar.add_css_class("past");
        }

        bar.set_class_active("negative", negative);

        bars.attach(&bar, index as i32, row, 1, height);

        // The price of the current hour, in the free rows above its bar
        if Some(index) == current && free >= 2 {
            let value = label(&decimal(price.price, 1), "energy__bar__value");
            value.set_halign(gtk::Align::Center);
            value.set_valign(gtk::Align::End);
            value.set_wrap(false);

            bars.attach(&value, index as i32, 0, 1, free);
        }

        let hour = if price.from.minute() == 0 && price.from.hour() % 3 == 0 {
            price.from.hour().to_string()
        } else {
            String::new()
        };

        let hour = label(&hour, "energy__bar__label");
        hour.set_halign(gtk::Align::Center);
        hours.append(&hour);
    }

    let plot = gtk::Overlay::new();
    plot.set_child(Some(&gridlines(step, above, below, digits)));
    plot.add_overlay(&bars);
    plot.set_measure_overlay(&bars, true);
    plot.set_vexpand(true);

    let baseline = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    baseline.add_css_class("energy__price__baseline");

    let under = gtk::Box::new(gtk::Orientation::Vertical, 0);
    under.set_hexpand(true);
    under.append(&baseline);
    under.append(&hours);

    // Only a baseline below zero gets a label
    let lowest = if below > 0 {
        format!("{} ct", decimal(-step * f64::from(below), digits))
    } else {
        String::new()
    };

    let bottom = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    bottom.append(&scale_label(&lowest));
    bottom.append(&under);

    let chart = gtk::Box::new(gtk::Orientation::Vertical, 12);
    chart.add_css_class("energy__card");
    chart.set_vexpand(true);
    chart.append(&header);
    chart.append(&plot);
    chart.append(&bottom);

    let section = gtk::Box::new(gtk::Orientation::Vertical, 0);
    section.append(&chart);

    Some(section)
}

fn key(swatch: Option<&str>, title: &str, value: &str) -> gtk::Box {
    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 8);

    if let Some(class) = swatch {
        let swatch = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        swatch.add_css_class("energy__swatch");
        swatch.add_css_class(class);
        swatch.set_valign(gtk::Align::Center);
        heading.append(&swatch);
    }

    heading.append(&label(title, "energy__key__title"));

    let value = label(value, "energy__key__value");
    value.set_wrap(false);

    let key = gtk::Box::new(gtk::Orientation::Vertical, 2);
    key.append(&heading);
    key.append(&value);

    key
}

/// Dashed lines with their values from the top down, solid at zero.
fn gridlines(step: f64, above: i32, below: i32, digits: usize) -> gtk::Box {
    let lines = gtk::Box::new(gtk::Orientation::Vertical, 0);
    lines.set_homogeneous(true);
    lines.set_hexpand(true);

    for segment in 0..above + below {
        let value = step * f64::from(above - segment);

        let line = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        line.set_hexpand(true);
        line.add_css_class("energy__price__gridline");
        line.set_class_active("zero", segment == above);

        let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        row.append(&scale_label(&format!("{} ct", decimal(value, digits))));
        row.append(&line);
        lines.append(&row);
    }

    lines
}

fn scale_label(text: &str) -> gtk::Label {
    let label = label(text, "energy__price__scale");
    label.set_valign(gtk::Align::Start);
    label.set_wrap(false);
    label.set_width_request(SCALE_WIDTH);

    label
}

/// The step and the segments above and below zero for the prices of the day.
fn scale(min: f64, max: f64) -> (f64, i32, i32) {
    let high = max.max(0.0);
    let low = (-min).max(0.0);
    let (step, _) = scale_step(high + low);

    let above = ((high / step).ceil() as i32).max(1);
    let below = (low / step).ceil() as i32;

    (step, above, below)
}

/// A step of 1, 2 or 5 times a power of ten and the segments it takes to reach `max`.
fn scale_step(max: f64) -> (f64, i32) {
    let max = max.max(1.0);
    let magnitude = 10f64.powf((max / f64::from(MAX_SEGMENTS)).log10().floor());

    for factor in [1.0, 2.0, 5.0, 10.0] {
        let step = factor * magnitude;
        let segments = (max / step).ceil() as i32;

        if segments <= MAX_SEGMENTS {
            return (step, segments.max(1));
        }
    }

    (20.0 * magnitude, 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_familiar_steps() {
        assert_eq!(scale_step(41.3), (10.0, 5));
        assert_eq!(scale_step(14.2), (5.0, 3));
        assert_eq!(scale_step(30.0), (10.0, 3));
        assert_eq!(scale_step(8.0), (2.0, 4));
    }

    #[test]
    fn makes_room_below_zero() {
        assert_eq!(scale(12.9, 41.3), (10.0, 5, 0));
        assert_eq!(scale(-4.1, 38.0), (10.0, 4, 1));
    }

    #[test]
    fn keeps_a_scale_without_positive_prices() {
        let (step, segments) = scale_step(-3.0);

        assert!((step - 0.2).abs() < 1e-9);
        assert_eq!(segments, 5);
    }
}
