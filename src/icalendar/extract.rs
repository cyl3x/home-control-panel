pub fn href(element: &xmltree::Element) -> Option<String> {
  element
    .get_child("href")
    .and_then(|e| e.get_text())
    .map(|e| e.to_string())
}

pub fn etag(element: &xmltree::Element) -> Option<String> {
  element
    .get_child("propstat")
    .and_then(|e| e.get_child("prop"))
    .and_then(|e| e.get_child("getetag"))
    .and_then(|e| e.get_text())
    .map(|e| e.to_string())
}

pub fn event_data(element: &xmltree::Element) -> Option<String> {
  element
    .get_child("propstat")
    .and_then(|e| e.get_child("prop"))
    .and_then(|e| e.get_child("calendar-data"))
    .and_then(|e| e.get_text())
    .map(|e| e.to_string())
}

pub fn calendar_name(element: &xmltree::Element) -> Option<String> {
  element
    .get_child("propstat")
    .and_then(|e| e.get_child("prop"))
    .and_then(|e| e.get_child("displayname"))
    .and_then(|e| e.get_text())
    .map(|e| e.to_string())
}

pub fn calendar_color(element: &xmltree::Element) -> Option<String> {
  element
    .get_child("propstat")
    .and_then(|e| e.get_child("prop"))
    .and_then(|e| e.get_child("calendar-color"))
    .and_then(|e| e.get_text())
    .map(|e| e.to_string())
}

pub fn is_calendar(element: &xmltree::Element) -> bool {
  element
    .get_child("propstat")
    .and_then(|e| e.get_child("prop"))
    .and_then(|e| e.get_child("resourcetype"))
    .map(|e| e.get_child("calendar").is_some())
    .unwrap_or(false)
}

pub fn calendar_supports_vevents(element: &xmltree::Element) -> bool {
  element
    .get_child("propstat")
    .and_then(|e| e.get_child("prop"))
    .and_then(|e| e.get_child("supported-calendar-component-set"))
    .map(|e| {
      e.children
        .iter()
        .filter_map(|c| c.as_element())
        .filter(|e| e.name == "comp")
        .filter_map(|e| e.attributes.get("name"))
        .any(|name| name == "VEVENT" || name == "VTODO")
    })
    .unwrap_or(false)
}