#![warn(
    clippy::all,
    // clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    // clippy::unwrap_used
)]
use app::App;
use clap::Parser;

pub use chrono::prelude::*;
pub use gtk::prelude::*;
pub use relm4::prelude::*;

mod app;
mod cli;
mod config;
pub mod calendar;
mod logger;
mod widgets;

static CSS: &str = include_str!("style.css");

fn main() {
  logger::init();

  let cli = cli::Cli::parse();

  let config = config::init(cli.config).expect("Could not load the configuration file");

  clapper::init().expect("Could not initialize the video player.");

  let app = RelmApp::new("cyl3x.home-control-panel");
  app.set_global_css(CSS);
  app.with_args(vec![]).run::<App>(config);
}
