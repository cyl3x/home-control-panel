use chrono::{DateTime, Local};

/// Errors that may occur while talking to the myVAILLANT API.
#[derive(Debug, Clone)]
pub enum Error {
    /// The request did not complete, e.g. DNS, TLS or a timeout
    Http(String),
    /// The API answered with a non-success status code
    Status {
        status: u16,
        url: String,
        body: String,
    },
    Parsing(String),
    Authentication(String),
    /// The API quota is used up, requests are paused until the given time
    QuotaExceeded(DateTime<Local>),
    /// The response was requested from the cache only, but is not cached
    NotCached(String),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Http(message) => write!(f, "HTTP error: {message}"),
            Self::Status { status, url, body } => {
                write!(f, "HTTP {status} calling \"{url}\": {body}")
            }
            Self::Parsing(message) => write!(f, "Parsing error: {message}"),
            Self::Authentication(message) => write!(f, "Authentication failed: {message}"),
            Self::QuotaExceeded(until) => write!(f, "Quota exceeded, paused until {until}"),
            Self::NotCached(url) => write!(f, "No cached response for \"{url}\""),
        }
    }
}

impl std::error::Error for Error {}

impl From<ureq::Error> for Error {
    fn from(e: ureq::Error) -> Self {
        match e {
            ureq::Error::Json(e) => Self::Parsing(e.to_string()),
            e => Self::Http(e.to_string()),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Parsing(e.to_string())
    }
}
