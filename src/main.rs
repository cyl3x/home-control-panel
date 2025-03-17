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
use iced::window::settings::PlatformSpecific;
use iced::{window, Settings, Theme};

mod app;
pub mod calendar;
mod cli;
pub mod config;
pub mod views;
pub mod widgets;

fn main() -> iced::Result {
    env_logger::builder().init();

    let cli = cli::Cli::parse();
    let config = config::init(cli.config).expect("Could not load the configuration file");

    iced::application::application("Home control panel", App::update, App::view)
        .subscription(App::subscription)
        .theme(|_| Theme::CatppuccinLatte)
        .scale_factor(|_| 1.5)
        .settings(Settings {
            fonts: vec![include_bytes!("./InterVariable.ttf").into()],
            default_font: iced::Font::with_name("Inter"),
            ..Default::default()
        })
        .window(window::Settings {
            platform_specific: PlatformSpecific {
                application_id: "de.cyl3x.home-control-panel".to_string(),
                ..Default::default()
            },
            ..Default::default()
        })
        .run_with(|| App::new(config))
}
