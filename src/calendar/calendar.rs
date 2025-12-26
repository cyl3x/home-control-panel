use std::str::FromStr;

use color::Rgba8;
use uuid::Uuid;

use super::extract;

#[derive(Clone, Debug)]
pub struct Calendar {
    pub uid: Uuid,
    pub url_str: String,
    pub name: String,
    pub color: Rgba8,
}

impl Calendar {
    pub fn from_xml(element: &xmltree::Element) -> Option<Self> {
        if !extract::is_calendar(element) || !extract::calendar_supports_vevents(element) {
            return None;
        }

        let href = extract::href(element)?;
        let name = extract::calendar_name(element)?;
        let color: Rgba8 = extract::calendar_color(element)
            .and_then(|color| color.try_into().ok())
            .unwrap_or_else(|| Rgba8::from_u32(0xdeb887));

        let uid = Uuid::new_v5(&Uuid::NAMESPACE_URL, href.as_bytes());

        Some(Self {
            uid,
            url_str: href,
            name,
            color,
        })
    }

    pub fn fg_color(&self) -> Rgba8 {
        fg_from_bg_w3c(&self.color).unwrap_or(Rgba8::from_u32(0x000000))
    }
}

impl PartialEq for Calendar {
    fn eq(&self, other: &Self) -> bool {
        self.uid == other.uid
    }
}

fn fg_from_bg_w3c(bg_color: &Rgba8) -> Option<Rgba8> {
    let linear: [f64; 4] = [bg_color.r.into() / 255, bg_color.g.into() / 255, bg_color.b.into() / 255, bg_color.a.into() / 255];
    let rgb = linear.map(|c| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });

    if rgb[0].mul_add(0.2126, rgb[1].mul_add(0.7152, rgb[2] * 0.0722)) > 0.179 {
        Some(Rgba8::from_u32(0x000000))
    } else {
        Some(Rgba8::from_u32(0xffffff))
    }
}
