// minicaldav: Small and easy CalDAV client.
// Copyright (C) 2022 Florian Loers
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! CalDAV client implementation using ureq.

use chrono::NaiveDate;
use ureq::Agent;
use url::Url;

pub enum Credentials {
  Basic(String, String),
  Bearer(String),
}

impl core::fmt::Debug for Credentials {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("<hidden>")
  }
}

#[derive(Debug)]
pub struct CaldavClient {
  credentials: Credentials,
  agent: Agent,
  base_url: Url,
}

impl CaldavClient {
  pub fn new(credentials: Credentials, base_url: Url) -> Self {
    Self {
      credentials,
      agent: Agent::new(),
      base_url,
    }
  }

  fn get_auth_header(&self) -> String {
    match &self.credentials {
      Credentials::Basic(username, password) => {
        format!(
          "Basic {}",
          base64::encode(format!("{}:{}", username, password))
        )
      }
      Credentials::Bearer(token) => format!("Bearer {}", token),
    }
  }

  /// Send a PROPFIND to the given url using the given HTTP Basic authorization and search the result XML for a value.
  /// # Arguments
  /// - client: ureq Agent
  /// - username: used for HTTP Basic auth
  /// - password: used for HTTP Basic auth
  /// - url: The caldav endpoint url
  /// - body: The CalDAV request body to send via PROPFIND
  /// - prop_path: The path in the response XML the get the XML text value from.
  /// - depth: Value for the Depth field
  pub fn propfind_get(
    &self,
    url: &Url,
    body: &str,
    prop_path: &[&str],
    depth: &str,
  ) -> Result<(String, xmltree::Element), Error> {
    let auth = self.get_auth_header();

    let content = self.agent
      .request("PROPFIND", url.as_str())
      .set("Authorization", &auth)
      .set("Content-Type", "application/xml")
      .set("Depth", depth)
      .send_bytes(body.as_bytes())?
      .into_string()
      .map_err(|e| Error {
        kind: ErrorKind::Parsing,
        message: e.to_string(),
      })?;

    log::trace!("CalDAV propfind response: {:?}", content);
    let reader = content.as_bytes();

    let root = xmltree::Element::parse(reader)?;
    let mut element = &root;
    let mut searched = 0;
    for prop in prop_path {
      for e in &element.children {
        if let Some(child) = e.as_element() {
          if child.name == *prop {
            searched += 1;
            element = child;
            break;
          }
        }
      }
    }

    if searched != prop_path.len() {
      Err(Error {
        kind: ErrorKind::Parsing,
        message: format!("Could not find data {:?} in PROPFIND response.", prop_path),
      })
    } else {
      Ok((
        element
          .get_text()
          .map(|s| s.to_string())
          .unwrap_or_else(|| "".to_string()),
        root,
      ))
    }
  }

  /// Get the CalDAV principal URL for the given credentials from the caldav server.
  pub fn get_principal_url(
    &self,
    url: &Url,
  ) -> Result<Url, Error> {
    let principal_url = self.propfind_get(
      url,
      USER_PRINCIPAL_REQUEST,
      &[
        "response",
        "propstat",
        "prop",
        "current-user-principal",
        "href",
      ],
      "0",
    )?
    .0;

    Ok(url.join(&principal_url)?)
  }

  /// Get the homeset url for the given credentials from the caldav server.
  pub fn get_home_set_url(&self, url: &Url) -> Result<Url, Error> {
    let principal_url = self.get_principal_url(url).unwrap_or_else(|_| url.clone());
    let homeset_url = self.propfind_get(
      &principal_url,
      HOMESET_REQUEST,
      &["response", "propstat", "prop", "calendar-home-set", "href"],
      "0",
    )?
    .0;

    Ok(url.join(&homeset_url)?)
  }

  /// Get calendars for the given credentials.
  pub fn get_calendars(&self) -> Result<Vec<CalendarRef>, Error> {
    let mut calendars = Vec::new();
    let result = match self.get_home_set_url(&self.base_url) {
      Ok(homeset_url) => self.propfind_get(
        &homeset_url,
        CALENDARS_REQUEST,
        &[],
        "1",
      ),
      Err(_e) => self.propfind_get(
        &self.base_url,
        CALENDARS_REQUEST,
        &[],
        "1",
      ),
    };

    let root = if result.is_err() {
      self.propfind_get(&self.base_url, CALENDARS_QUERY, &[], "1")?.1
    } else {
      result?.1
    };

    for response in &root.children {
      if let Some(response) = response.as_element() {
        let name = response
          .get_child("propstat")
          .and_then(|e| e.get_child("prop"))
          .and_then(|e| e.get_child("displayname"))
          .and_then(|e| e.get_text());
        let color = response
          .get_child("propstat")
          .and_then(|e| e.get_child("prop"))
          .and_then(|e| e.get_child("calendar-color"))
          .and_then(|e| e.get_text());
        let is_calendar = response
          .get_child("propstat")
          .and_then(|e| e.get_child("prop"))
          .and_then(|e| e.get_child("resourcetype"))
          .map(|e| e.get_child("calendar").is_some())
          .unwrap_or(false);
        let supports_vevents = response
          .get_child("propstat")
          .and_then(|e| e.get_child("prop"))
          .and_then(|e| e.get_child("supported-calendar-component-set"))
          .map(|e| {
            for c in &e.children {
              if let Some(child) = c.as_element() {
                if child.name == "comp" {
                  if let Some(name) = child.attributes.get("name") {
                    if (name == "VEVENT") || (name == "VTODO") {
                      return true;
                    }
                  }
                }
              }
            }
            false
          })
          .unwrap_or(false);
        let href = response.get_child("href").and_then(|e| e.get_text());

        if !is_calendar || !supports_vevents {
          continue;
        }
        if let Some((href, name)) = href.and_then(|href| name.map(|name| (href, name))) {
          if let Ok(url) = self.base_url.join(&href) {
            calendars.push(CalendarRef {
              url,
              name: name.to_string(),
              color: color.map(|c| c.into()),
            })
          } else {
            log::error!("Could not parse url: {}/{}", &self.base_url, href);
          }
        } else {
          continue;
        }
      }
    }

    Ok(calendars)
  }

  /// Get ICAL formatted events from the CalDAV server.
  pub fn get_events(
      &self,
      request: String,
      calendar_url: &Url,
  ) -> Result<Vec<EventRef>, Error> {
    let auth = self.get_auth_header();
    let content = self.agent
      .request("REPORT", calendar_url.as_str())
      .set("Authorization", &auth)
      .set("Depth", "1")
      .set("Content-Type", "application/xml")
      .send_bytes(request.as_bytes())?
      .into_string()
      .map_err(|e| Error {
        kind: ErrorKind::Parsing,
        message: e.to_string(),
      })?;

    log::trace!("Read CalDAV events: {:?}", content);
    let reader = content.as_bytes();

    let root = xmltree::Element::parse(reader)?;
    let mut events = Vec::new();
    for c in &root.children {
      if let Some(child) = c.as_element() {
        let href = child.get_child("href").and_then(|e| e.get_text());
        let etag = child
          .get_child("propstat")
          .and_then(|e| e.get_child("prop"))
          .and_then(|e| e.get_child("getetag"))
          .and_then(|e| e.get_text())
          .map(|e| e.to_string());
        let data = child
          .get_child("propstat")
          .and_then(|e| e.get_child("prop"))
          .and_then(|e| e.get_child("calendar-data"))
          .and_then(|e| e.get_text());
        if href.is_none() || etag.is_none() || data.is_none() {
          continue;
        }

        if let Some((href, data)) = href.and_then(|href| data.map(|data| (href, data))) {
          if let Ok(url) = self.base_url.join(&href) {
            events.push(EventRef {
              url,
              data: data.to_string(),
              etag,
            })
          } else {
            log::error!("Could not parse url {}/{}", &self.base_url, href)
          }
        }
      }
    }

    Ok(events)
  }

  /// Get ICAL formatted todos from the CalDAV server.
  pub fn get_todos(
      &self,
      request: String,
      calendar_ref: &CalendarRef,
  ) -> Result<Vec<EventRef>, Error> {
    let auth = self.get_auth_header();

    let content = self.agent
      .request("REPORT", calendar_ref.url.as_str())
      .set("Authorization", &auth)
      .set("Depth", "1")
      .set("Content-Type", "application/xml")
      .send_bytes(request.as_bytes())?
      .into_string()
      .map_err(|e| Error {
        kind: ErrorKind::Parsing,
        message: e.to_string(),
      })?;

    log::trace!("Read CalDAV events: {:?}", content);
    let reader = content.as_bytes();

    let root = xmltree::Element::parse(reader)?;
    let mut todos = Vec::new();
    for c in &root.children {
      if let Some(child) = c.as_element() {
        let href = child.get_child("href").and_then(|e| e.get_text());
        let etag = child
          .get_child("propstat")
          .and_then(|e| e.get_child("prop"))
          .and_then(|e| e.get_child("getetag"))
          .and_then(|e| e.get_text())
          .map(|e| e.to_string());
        let data = child
          .get_child("propstat")
          .and_then(|e| e.get_child("prop"))
          .and_then(|e| e.get_child("calendar-data"))
          .and_then(|e| e.get_text());
        if href.is_none() || etag.is_none() || data.is_none() {
          continue;
        }

        if let Some((href, data)) = href.and_then(|href| data.map(|data| (href, data))) {
          if let Ok(url) = self.base_url.join(&href) {
            todos.push(EventRef {
              url,
              data: data.to_string(),
              etag,
            })
          } else {
            log::error!("Could not parse url {}/{}", &self.base_url, href)
          }
        }
      }
    }

    Ok(todos)
  }

  /// Save the given event on the CalDAV server.
  /// If no event for the events url exist it will create a new event.
  /// Otherwise this is an update operation.
  pub fn save_event(&self, event_ref: EventRef) -> Result<EventRef, Error> {
    let auth = self.get_auth_header();

    let response = self.agent
      .put(event_ref.url.as_str())
      .set("Content-Type", "text/calendar")
      .set("Content-Length", &event_ref.data.len().to_string())
      .set("Authorization", &auth)
      .send(event_ref.data.as_bytes())?;

    if let Some(etag) = response.header("ETag") {
      Ok(EventRef {
        etag: Some(etag.into()),
        ..event_ref
      })
    } else {
      Ok(EventRef {
        etag: None,
        ..event_ref
      })
    }
  }

  /// Delete the given event from the CalDAV server.
  pub fn remove_event(&self, event_ref: EventRef) -> Result<(), Error> {
    let auth = self.get_auth_header();

    let _response = self.agent
      .delete(event_ref.url.as_str())
      .set("Authorization", &auth)
      .call()?;

    Ok(())
  }
}

pub struct DavQuery {
  
}

pub static USER_PRINCIPAL_REQUEST: &str = r#"
<d:propfind xmlns:d="DAV:">
  <d:prop>
    <d:current-user-principal />
  </d:prop>
</d:propfind>
"#;

pub static CALENDARS_REQUEST: &str = r#"
<d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" >
  <d:prop>
    <d:displayname />
    <d:resourcetype />
    <calendar-color xmlns="http://apple.com/ns/ical/" />
    <c:supported-calendar-component-set /> 
  </d:prop>
</d:propfind>
"#;

pub static CALENDARS_QUERY: &str = r#"
<c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:prop>
    <d:getetag />
    <d:displayname />
    <calendar-color xmlns="http://apple.com/ns/ical/" />
    <d:resourcetype />
    <c:supported-calendar-component-set />
  </d:prop>
  <c:filter>
    <c:comp-filter name="VCALENDAR" />
  </c:filter>
</c:calendar-query>
"#;

pub static HOMESET_REQUEST: &str = r#"
<d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" >
  <d:self/>
  <d:prop>
    <c:calendar-home-set />
  </d:prop>
</d:propfind>
"#;

pub fn filter_time_range(start: NaiveDate, end: NaiveDate) -> String {
  format!(r#"<c:time-range start="{}" end="{}" />"#, start.format("%Y%m%dT000000Z"), end.format("%Y%m%dT000000Z"))
}

pub fn request_event(filter: String) -> String {
  format!(r#"
<c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:prop>
    <d:getetag />
    <c:calendar-data />
  </d:prop>
  <c:filter>
    <c:comp-filter name="VCALENDAR">
      <c:comp-filter name="VEVENT">
        {}
      </c:comp-filter>
    </c:comp-filter>
  </c:filter>
</c:calendar-query>
  "#, filter)
}

pub fn request_todos(filter: String) -> String {
  format!(r#"<c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:prop>
    <d:getetag />
    <c:calendar-data />
  </d:prop>
  <c:filter>
    <c:comp-filter name="VCALENDAR">
      <c:comp-filter name="VTODO">
        {}
      </c:comp-filter>
    </c:comp-filter>
  </c:filter>
</c:calendar-query>"#, filter)
}

#[derive(Clone, Debug)]
pub struct CalendarRef {
  pub url: Url,
  pub name: String,
  pub color: Option<String>,
}

#[derive(Clone, Debug)]
pub struct EventRef {
  pub etag: Option<String>,
  pub url: Url,
  pub data: String,
}

/// Errors that may occur during CalDAV operations.
#[derive(Debug)]
pub struct Error {
  pub kind: ErrorKind,
  pub message: String,
}

#[derive(Debug)]
pub enum ErrorKind {
  Http,
  Parsing,
}

impl From<ureq::Error> for Error {
  fn from(e: ureq::Error) -> Self {
    Self {
      kind: ErrorKind::Http,
      message: format!("{:?}", e),
    }
  }
}

impl From<xmltree::ParseError> for Error {
  fn from(e: xmltree::ParseError) -> Self {
    Self {
      kind: ErrorKind::Parsing,
      message: e.to_string(),
    }
  }
}

impl From<url::ParseError> for Error {
  fn from(e: url::ParseError) -> Self {
    Self {
      kind: ErrorKind::Parsing,
      message: e.to_string(),
    }
  }
}
