use std::collections::BTreeMap;

use chrono::NaiveDate;
use ureq::Agent;
use url::Url;
use uuid::Uuid;
use base64::prelude::*;

use crate::config::Config;
use crate::icalendar::event_builder::EventBuilder;
use crate::icalendar::{Calendar, Event, EVENT_DEFAULT_COLOR};

#[derive(Clone)]
pub enum Credentials {
  Basic(String, String),
  Bearer(String),
}

impl From<&Config> for Credentials {
  fn from(config: &Config) -> Self {
    Self::Basic(
      config.ical.username.clone(),
      config.ical.password.as_ref().unwrap().clone(),
    )
  }
}

impl core::fmt::Debug for Credentials {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("<hidden>")
  }
}

#[derive(Debug, Clone)]
pub struct Client {
  credentials: Credentials,
  agent: Agent,
  base_url: Url,
}

impl Client {
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
          BASE64_STANDARD.encode(format!("{username}:{password}"))
        )
      }
      Credentials::Bearer(token) => format!("Bearer {token}"),
    }
  }

  /// Send a PROPFIND to the given url using the given HTTP Basic authorization and search the result XML for a value.
  /// 
  /// # Errors
  /// Returns an error if the request or the XML parsing fails.
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

    // log::trace!("CalDAV propfind response: {:?}", content);
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

    if searched == prop_path.len() {
      Ok((
        element
          .get_text().map_or_else(String::new, |s| s.to_string()),
        root,
      ))
    } else {
      Err(Error {
        kind: ErrorKind::Parsing,
        message: format!("Could not find data {prop_path:?} in PROPFIND response."),
      })
    }
  }

  /// Get the `CalDAV` principal URL for the given credentials from the caldav server.
  /// 
  /// # Errors
  /// Returns an error if the request or the XML parsing fails.
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
  /// 
  /// # Errors
  /// Returns an error if the request or the XML parsing fails.
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
  /// 
  /// # Errors
  /// Returns an error if the request or the XML parsing fails.
  pub fn get_calendars(&self) -> Result<BTreeMap<Uuid, Calendar>, Error> {
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

    let calendars = root.children
      .iter()
      .filter_map(|c| c.as_element())
      .filter_map(Calendar::from_xml)
      .filter_map(|mut calendar| {
        self.base_url.join(&calendar.url_str).ok()?.as_str().clone_into(&mut calendar.url_str);
        Some((calendar.uid, calendar))
      })
      .collect();

    Ok(calendars)
  }

  /// Get ICAL formatted events from the `CalDAV` server.
  /// 
  /// # Errors
  /// Returns an error if the request or the XML parsing fails.
  pub fn get_events(
      &self,
      request: &String,
      calendar_ref: &Calendar,
  ) -> Result<BTreeMap<Uuid, Event>, Error> {
    let auth = self.get_auth_header();
    let content = self.agent
      .request("REPORT", calendar_ref.url_str.as_str())
      .set("Authorization", &auth)
      .set("Depth", "1")
      .set("Content-Type", "application/xml")
      .send_bytes(request.as_bytes())?
      .into_string()
      .map_err(|e| Error {
        kind: ErrorKind::Parsing,
        message: e.to_string(),
      })?;

    let reader = content.as_bytes();
    let root = xmltree::Element::parse(reader)?;

    let events = root.children
        .iter()
        .filter_map(|c| c.as_element())
        .map(|element| EventBuilder::from(element)
            .set_calendar_uid_opt(Some(calendar_ref.uid))
            .set_color_opt(calendar_ref.color.clone())
            .with_default_color(EVENT_DEFAULT_COLOR)
            .with_base_url(&self.base_url)
            .build()
        )
        .filter_map(|result| match result {
            Ok(event) => Some((event.uid, event)),
            Err(e) => {
              log::error!("Error parsing event: {:?}", e);
              None
            }
        })
        .collect();

    Ok(events)
  }

  /// Get ICAL formatted todos from the `CalDAV` server.
  /// 
  /// # Errors
  /// Returns an error if the request or the XML parsing fails.
  pub fn get_todos(
      &self,
      request: &str,
      calendar_ref: &Calendar,
  ) -> Result<BTreeMap<Uuid, Event>, Error> {
    let auth = self.get_auth_header();

    let content = self.agent
      .request("REPORT", calendar_ref.url_str.as_str())
      .set("Authorization", &auth)
      .set("Depth", "1")
      .set("Content-Type", "application/xml")
      .send_bytes(request.as_bytes())?
      .into_string()
      .map_err(|e| Error {
        kind: ErrorKind::Parsing,
        message: e.to_string(),
      })?;

    let reader = content.as_bytes();
    let root = xmltree::Element::parse(reader)?;

    let todos = root.children
      .iter()
      .filter_map(|c| c.as_element())
      .map(|element| EventBuilder::from(element)
          .set_calendar_uid_opt(Some(calendar_ref.uid))
          .set_color_opt(calendar_ref.color.clone())
          .with_default_color(EVENT_DEFAULT_COLOR)
          .with_base_url(&self.base_url)
          .build()
      )
      .filter_map(|result| match result {
          Ok(event) => Some((event.uid, event)),
          Err(e) => {
            log::error!("Error parsing event: {:?}", e);
            None
          }
      })
      .collect();

    Ok(todos)
  }

  /// Save the given event on the `CalDAV` server.
  /// If no event for the events url exist it will create a new event.
  /// Otherwise this is an update operation.
  // pub fn save_event(&self, event: &mut Event) -> Result<(), Error> {
  //   let auth = self.get_auth_header();

  //   let response = self.agent
  //     .put(event.url.as_str())
  //     .set("Content-Type", "text/calendar")
  //     .set("Content-Length", &event.to_string().len().to_string())
  //     .set("Authorization", &auth)
  //     .send(event.to_string().as_bytes())?;

  //   if let Some(etag) = response.header("ETag") {
  //     event.etag(Some(etag.to_string()));
  //   } else {
  //     event.etag(None);

  //     return Err(Error {
  //       kind: ErrorKind::Parsing,
  //       message: format!("No ETag in response for event: {:?}", event),
  //     });
  //   }

  //   Ok(())
  // }

  /// Delete the given event from the `CalDAV` server.
  /// 
  /// # Errors
  /// Returns an error if the request fails.
  pub fn remove_event(&self, event: &Event) -> Result<(), Error> {
    let auth = self.get_auth_header();

    let _response = self.agent
      .delete(event.url.as_str())
      .set("Authorization", &auth)
      .call()?;

    Ok(())
  }
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

pub fn request_event(filter: &str) -> String {
  format!(r#"
<c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:prop>
    <d:getetag />
    <c:calendar-data />
  </d:prop>
  <c:filter>
    <c:comp-filter name="VCALENDAR">
      <c:comp-filter name="VEVENT">
        {filter}
      </c:comp-filter>
    </c:comp-filter>
  </c:filter>
</c:calendar-query>
  "#)
}

pub fn request_todos(filter: &str) -> String {
  format!(r#"<c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:prop>
    <d:getetag />
    <c:calendar-data />
  </d:prop>
  <c:filter>
    <c:comp-filter name="VCALENDAR">
      <c:comp-filter name="VTODO">
        {filter}
      </c:comp-filter>
    </c:comp-filter>
  </c:filter>
</c:calendar-query>"#)
}

/// Errors that may occur during `CalDAV` operations.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Error {
  pub kind: ErrorKind,
  pub message: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ErrorKind {
  Http,
  Parsing,
}

impl From<ureq::Error> for Error {
  fn from(e: ureq::Error) -> Self {
    Self {
      kind: ErrorKind::Http,
      message: format!("{e:?}"),
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
