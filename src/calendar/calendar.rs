use std::str::FromStr;

use iced::color;
use uuid::Uuid;

use super::extract;

#[derive(Clone, Debug)]
pub struct Calendar {
    pub uid: Uuid,
    pub url_str: String,
    pub name: String,
    pub color: iced::Color,
}

impl Calendar {
    pub fn from_xml(element: &xmltree::Element) -> Option<Self> {
        if !extract::is_calendar(element) || !extract::calendar_supports_vevents(element) {
            return None;
        }

        let href = extract::href(element)?;
        let name = extract::calendar_name(element)?;
        let color = extract::calendar_color(element)
            .and_then(|color| iced::Color::from_str(&color).ok())
            .unwrap_or_else(|| color!(0xdeb887));

        let uid = Uuid::new_v5(&Uuid::NAMESPACE_URL, href.as_bytes());

        Some(Self {
            uid,
            url_str: href,
            name,
            color,
        })
    }

    pub fn fg_color(&self) -> iced::Color {
        fg_from_bg_w3c(&self.color).unwrap_or(iced::Color::BLACK)
    }
}

impl PartialEq for Calendar {
    fn eq(&self, other: &Self) -> bool {
        self.uid == other.uid
    }
}

fn fg_from_bg_w3c(bg_color: &iced::Color) -> Option<iced::Color> {
    let rgb = bg_color.into_linear().map(|c| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });

    if rgb[0].mul_add(0.2126, rgb[1].mul_add(0.7152, rgb[2] * 0.0722)) > 0.179 {
        Some(iced::Color::BLACK)
    } else {
        Some(iced::Color::WHITE)
    }
}
