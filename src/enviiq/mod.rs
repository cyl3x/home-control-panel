//! Client for the Envi IQ cloud API of Environ Energy, which has no public documentation.
//!
//! The endpoints are taken from the Envi IQ app, the login from the installer UI served by
//! the Envi IQ box. Both use the same Cognito user pool.

mod auth;
mod client;
mod error;
mod models;
mod service;

pub use client::Client;
pub use error::Error;
pub use models::{Measurement, Price, Totals};
pub use service::{Overview, Service};
