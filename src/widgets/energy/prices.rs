use super::ui::{detail, label};
use crate::enviiq::Price;
use crate::prelude::*;

/// Smallest height the chart is useful at, above that it grows with the page
const CHART_MIN_HEIGHT: i32 = 160;

/// Rows the bars are measured in. Equally high rows let the bars keep their proportions
/// at any height, which a fixed pixel height would not.
const BAR_ROWS: i32 = 48;

/// Segments the chart is divided into, which sets both the gridlines and the scale ticks
const TICKS: i32 = 4;

/// Width of the ct scale left of the bars
const SCALE_WIDTH: i32 = 56;

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

    let summary = match (current, cheapest) {
        (Some(current), Some(cheapest)) if cheapest == current => {
            format!("Jetzt {:.1} ct · am günstigsten", prices[current].price)
        }
        (Some(current), Some(cheapest)) => format!(
            "Jetzt {:.1} ct · am günstigsten {} mit {:.1} ct",
            prices[current].price,
            prices[cheapest].from.format("%H:%M"),
            prices[cheapest].price
        ),
        _ => format!("{min:.1} – {max:.1} ct/kWh"),
    };

    let title = label("Börsenpreis heute", "energy__section__title");
    title.set_hexpand(true);

    let summary = detail(&summary, None);
    summary.set_valign(gtk::Align::End);
    summary.set_wrap(false);

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    header.append(&title);
    header.append(&summary);

    let bars = gtk::Grid::builder()
        .row_homogeneous(true)
        .column_homogeneous(true)
        .column_spacing(6)
        .height_request(CHART_MIN_HEIGHT)
        .vexpand(true)
        .hexpand(true)
        .build();

    let hours = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    hours.set_homogeneous(true);

    for (index, price) in prices.iter().enumerate() {
        // Negative prices get the lowest bar
        let ratio = if max > 0.0 {
            (price.price.max(0.0) / max).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let filled = ((ratio * f64::from(BAR_ROWS)).round() as i32).clamp(1, BAR_ROWS);
        let empty = BAR_ROWS - filled;

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

        bars.attach(&bar, index as i32, empty, 1, filled);

        // The price of the current hour sits right above its bar, so it can be read off.
        // It spans the free rows, so it never forces the chart to grow.
        if Some(index) == current && empty >= 2 {
            let value = label(&format!("{:.1}", price.price), "energy__bar__value");
            value.set_halign(gtk::Align::Center);
            value.set_valign(gtk::Align::End);
            value.set_wrap(false);

            bars.attach(&value, index as i32, 0, 1, empty);
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

    // The gridlines behind the bars and the scale beside them are divided into the same
    // number of segments, so a label always sits on its line
    let plot = gtk::Overlay::new();
    plot.set_child(Some(&gridlines()));
    plot.add_overlay(&bars);
    plot.set_measure_overlay(&bars, true);
    plot.set_vexpand(true);

    let baseline = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    baseline.add_css_class("energy__price__baseline");

    let plot_column = gtk::Box::new(gtk::Orientation::Vertical, 0);
    plot_column.set_hexpand(true);
    plot_column.set_vexpand(true);
    plot_column.append(&plot);
    plot_column.append(&baseline);
    plot_column.append(&hours);

    let scale = scale(max);
    scale.set_vexpand(true);

    let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    row.set_vexpand(true);
    row.append(&scale);
    row.append(&plot_column);

    let chart = gtk::Box::new(gtk::Orientation::Vertical, 12);
    chart.add_css_class("energy__card");
    chart.set_vexpand(true);
    chart.append(&header);
    chart.append(&row);

    let section = gtk::Box::new(gtk::Orientation::Vertical, 0);
    section.append(&chart);

    Some(section)
}

/// Dashed lines across the chart, one per segment.
fn gridlines() -> gtk::Box {
    let lines = gtk::Box::new(gtk::Orientation::Vertical, 0);
    lines.set_homogeneous(true);
    lines.set_hexpand(true);

    for _ in 0..TICKS {
        let line = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        line.add_css_class("energy__price__gridline");
        lines.append(&line);
    }

    lines
}

/// The ct values of the gridlines, counting down from the highest price of the day.
fn scale(max: f64) -> gtk::Box {
    let scale = gtk::Box::new(gtk::Orientation::Vertical, 0);
    scale.set_homogeneous(true);
    scale.set_width_request(SCALE_WIDTH);

    for tick in 0..TICKS {
        let value = max * f64::from(TICKS - tick) / f64::from(TICKS);

        let label = label(&format!("{value:.0} ct"), "energy__price__scale");
        label.set_valign(gtk::Align::Start);
        label.set_wrap(false);

        let cell = gtk::Box::new(gtk::Orientation::Vertical, 0);
        cell.append(&label);
        scale.append(&cell);
    }

    scale
}
