#![warn(
    clippy::all,
    // clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    // clippy::unwrap_used
)]
use std::path::PathBuf;

use app::App;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, CssProvider};

pub use chrono::prelude::*;

mod app;
pub mod calendar;
pub mod config;
// pub mod views;
// pub mod widgets;

fn main() {
    env_logger::builder().init();

    let mut args = std::env::args().collect::<Vec<_>>();

    if args.len() < 2 {
        eprintln!("Usage: {} <config> <other-args>", args[0]);
        std::process::exit(1);
    }

    let config_path: PathBuf = PathBuf::try_from(args.remove(1)).expect("Invalid config path");
    let config = config::init(config_path).expect("Could not load the configuration file");

    let app = Application::builder()
        .application_id("de.cyl3x.home-control-panel")
        .build();

    app.connect_startup(|_| {
        let provider = CssProvider::new();
        provider.load_from_string(include_str!("style.css"));

        gtk::style_context_add_provider_for_display(
            &Display::default().expect("Could not connect to a display."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    app.connect_activate(|app| {
        App::new(app);
    });

    let exit_code = app.run_with_args(&args);
    std::process::exit(exit_code.into());
}
