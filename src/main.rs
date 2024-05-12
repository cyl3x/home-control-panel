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
pub use relm4::prelude::*;

mod app;
pub mod calendar;
mod cli;
mod config;
// mod core;
mod components;

fn main() {
  let cli = cli::Cli::parse();

  env_logger::init();

  let config = config::init(cli.config).unwrap();

  clapper::init().expect("Could not initialize the video player.");

  let app = RelmApp::new("cyl3x.home-dashboard");

  let css_provider = gtk::CssProvider::new();
  css_provider.load_from_string(include_str!("style.css"));
  gtk::style_context_add_provider_for_display(
    &Display::default().expect("Could not connect to a display."),
    &css_provider,
    gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
  );

  app.run::<App>(config);
}
