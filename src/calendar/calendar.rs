use palette::rgb::Rgb;
use uuid::Uuid;

use crate::calendar::Color;

use super::extract;

#[derive(Clone, Debug)]
pub struct Calendar {
    pub uid: Uuid,
    pub url_str: String,
    pub name: String,
    pub color: Color,
}

impl Calendar {
    pub fn from_xml(element: &xmltree::Element) -> Option<Self> {
        if !extract::is_calendar(element) || !extract::calendar_supports_vevents(element) {
            return None;
        }

        let href = extract::href(element)?;

        let uid = Uuid::new_v5(&Uuid::NAMESPACE_URL, href.as_bytes());

        Some(Self {
            uid,
            url_str: href,
            name: extract::calendar_name(element)?,
            color: extract::calendar_color(element)
                .and_then(|color| color.parse().ok())
                .unwrap_or_else(|| Rgb::new(222, 184, 135)),
        })
    }

    pub fn css_color(&self) -> String {
        format!(
            "rgb({}, {}, {})",
            self.color.red, self.color.green, self.color.blue
        )
    }

    pub fn fg_color(&self) -> Color {
        fg_from_bg_w3c(&self.color).unwrap_or(Rgb::new(0, 0, 0))
    }
}

impl PartialEq for Calendar {
    fn eq(&self, other: &Self) -> bool {
        self.uid == other.uid
    }
}

fn fg_from_bg_w3c(bg_color: &Color) -> Option<Color> {
    let linear = [
        f32::from(bg_color.red) / 255.0,
        f32::from(bg_color.green) / 255.0,
        f32::from(bg_color.blue) / 255.0,
    ];
    let rgb = linear.map(|c| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });

    if rgb[0].mul_add(0.2126, rgb[1].mul_add(0.7152, rgb[2] * 0.0722)) > 0.179 {
        Some(Rgb::new(0, 0, 0))
    } else {
        Some(Rgb::new(255, 255, 255))
    }
}
