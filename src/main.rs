#![warn(
    clippy::all,
    // clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    // clippy::unwrap_used
)]
use app::App;
use clap::Parser;
use gtk::gdk::Display;

pub use chrono::prelude::*;
pub use gtk::prelude::*;
use logger::LogExt;
pub use relm4::prelude::*;

mod app;
pub mod calendar;
mod cli;
mod config;
pub mod icalendar;
mod components;
mod logger;

static CSS: &str = include_str!("style.css");

fn main() {
  logger::init();

  let cli = cli::Cli::parse();

  let config = config::init(cli.config).log_error("Could not load the configuration file").unwrap();

  clapper::init().expect("Could not initialize the video player.");

  let app = RelmApp::new("cyl3x.home-dashboard");

  let css_provider = gtk::CssProvider::new();
  css_provider.load_from_string(CSS);
  gtk::style_context_add_provider_for_display(
    &Display::default().expect("Could not connect to a display."),
    &css_provider,
    gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
  );

  app.with_args(vec![]).run::<App>(config);
}
