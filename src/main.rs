#![warn(
    clippy::all,
    // clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    // clippy::unwrap_used
)]
use std::path::PathBuf;

use app::App;
use gtk::{glib, prelude::*};
use gtk::{Application, CssProvider, gdk::Display};

pub use chrono::prelude::*;

mod app;
pub mod calendar;
pub mod config;
// pub mod views;
pub mod messaging;
pub mod widgets;
pub mod prelude;

fn main() {
    env_logger::builder().init();
    clapper::init().unwrap();

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
            gtk::STYLE_PROVIDER_PRIORITY_USER,
        );
    });

    app.connect_activate(move |app| {
        let mut app = App::new(&app, &config);

        glib::spawn_future_local(async move {
            while let Ok(message) = messaging::receiver().recv().await {
                app.update(message);
            }
        });
    });

    let exit_code = app.run_with_args(&args);
    std::process::exit(exit_code.into());
}
