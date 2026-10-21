//! Client for the myVAILLANT cloud API, which has no official documentation.
//!
//! Login flow, endpoints and quirks follow the community library myPyllant
//! (<https://github.com/signalkraft/myPyllant>, v0.9.18).

mod auth;
mod cache;
mod client;
mod command;
mod error;
mod models;
mod service;

pub use client::Client;
pub use command::{Action, Command, Setpoint};
pub use error::Error;
pub use service::{Circuit, HeatingSystem, HotWater, Service, Zone};
