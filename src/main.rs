#![warn(
    clippy::all,
    // clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    // clippy::unwrap_used
)]
use std::path::PathBuf;

use app::App;
use gtk::glib;
use gtk::{Application, CssProvider, gdk::Display};

use crate::prelude::*;

mod app;
pub mod calendar;
pub mod config;
pub mod gtk_ext;
pub mod messaging;
pub mod prelude;
pub mod widgets;

fn main() {
    env_logger::builder().init();
    clapper::init().unwrap();

    let mut args = std::env::args().collect::<Vec<_>>();

    if args.len() < 2 {
        eprintln!("Usage: {} <config> <other-args>", args[0]);
        std::process::exit(1);
    }

    let config_path: PathBuf = PathBuf::from(args.remove(1));
    let config = match config::init(config_path) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load config: {e}");
            std::process::exit(1);
        }
    };

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
        let mut app = App::new(app, &config);

        glib::spawn_future_local(async move {
            while let Ok(message) = messaging::receiver().recv().await {
                app.update(message);
            }
        });

        glib::unix_signal_add_local(10 /* USR1 */, || {
            log::info!("Received USR1 signal: selecting now");

            messaging::send_message(messaging::CalendarMessage::SelectNow);

            glib::ControlFlow::Continue
        });
    });

    let exit_code = app.run_with_args(&args);
    std::process::exit(exit_code.into());
}

pub fn remove_source(id: Option<glib::SourceId>) {
    if let Some(id) = id
        && glib::MainContext::default()
            .find_source_by_id(&id)
            .is_some()
    {
        id.remove();
    }
}
