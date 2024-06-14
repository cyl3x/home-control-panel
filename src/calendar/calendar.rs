use uuid::Uuid;

use super::extract;

#[derive(Clone, Debug)]
pub struct Calendar {
  pub uid: Uuid,
  pub url_str: String,
  pub name: String,
  pub color: Option<String>,
}

impl Calendar {
  pub fn from_xml(element: &xmltree::Element) -> Option<Self> {
    if !extract::is_calendar(element) || !extract::calendar_supports_vevents(element) {
      return None;
    }

    let href = extract::href(element)?;
    let name = extract::calendar_name(element)?;
    let color = extract::calendar_color(element);

    let uid = Uuid::new_v5(&Uuid::NAMESPACE_URL, href.as_bytes());

    Some(Self { uid, url_str: href, name, color })
  }

  pub fn color(&self) -> &str {
    self.color.as_deref().unwrap_or("#deb887")
  }
}
