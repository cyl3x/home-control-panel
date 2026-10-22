use super::ui::{detail, label};
use crate::enviiq::Price;
use crate::prelude::*;

/// Height of the highest bar of the chart
const CHART_HEIGHT: i32 = 96;

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
            "Jetzt {:.1} ct · am günstigsten {}",
            prices[current].price,
            prices[cheapest].from.format("%H:%M")
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

    let bars = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    bars.set_homogeneous(true);
    bars.set_height_request(CHART_HEIGHT);

    let hours = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    hours.set_homogeneous(true);

    for (index, price) in prices.iter().enumerate() {
        // Negative prices get the lowest bar
        let height = if max > 0.0 {
            price.price.max(0.0) / max * f64::from(CHART_HEIGHT)
        } else {
            0.0
        };

        let bar = gtk::Box::new(gtk::Orientation::Vertical, 0);
        bar.add_css_class("energy__bar");
        bar.set_valign(gtk::Align::End);
        bar.set_height_request((height.round() as i32).max(2));

        if Some(index) == current {
            bar.add_css_class("current");
        } else if Some(index) == cheapest {
            bar.add_css_class("cheapest");
        } else if price.to <= now {
            bar.add_css_class("past");
        }

        bars.append(&bar);

        let hour = if price.from.minute() == 0 && price.from.hour() % 3 == 0 {
            price.from.hour().to_string()
        } else {
            String::new()
        };

        let hour = label(&hour, "energy__bar__label");
        hour.set_halign(gtk::Align::Center);
        hours.append(&hour);
    }

    let chart = gtk::Box::new(gtk::Orientation::Vertical, 4);
    chart.add_css_class("energy__card");
    chart.append(&bars);
    chart.append(&hours);

    let section = gtk::Box::new(gtk::Orientation::Vertical, 16);
    section.append(&header);
    section.append(&chart);

    Some(section)
}
