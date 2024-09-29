use std::time::Instant;

use chrono::DateTime;
use chrono_tz::{Europe, Tz};
use iced::font;
use iced::widget::{column, container, text};
use iced::{time, Alignment, Color, Length};

use crate::config;
use crate::widgets::interaction_tracker::InteractionTracker;

pub struct Screensaver {
    config: config::Screensaver,
    pub state: State,
    last_interaction: Instant,
    now: DateTime<Tz>,
    dim: bool,
}

#[derive(Clone, Copy)]
pub enum State {
    Active,
    Inactive,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick(Instant),
    Interact,
}

impl Screensaver {
    pub fn new(config: config::Screensaver) -> Self {
        Self {
            config,
            state: State::Inactive,
            last_interaction: time::Instant::now(),
            now: chrono::Utc::now().with_timezone(&Europe::Berlin),
            dim: false,
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        match self.state {
            State::Inactive => time::every(
                time::Duration::from_secs(self.config.timeout) - self.last_interaction.elapsed(),
            )
            .map(Message::Tick),
            State::Active => time::every(time::Duration::from_secs(1)).map(Message::Tick),
        }
    }

    pub fn view_active(&self) -> iced::Element<Message> {
        let clock = match self.dim {
            true => column![],
            false => column![
                text(self.time())
                    .size(90)
                    .color(Color::from_rgb8(127, 127, 127))
                    .font(font::Font {
                        family: font::Family::Name("Inter"),
                        weight: font::Weight::Bold,
                        ..Default::default()
                    }),
                text(self.date())
                    .size(60)
                    .color(Color::from_rgb8(127, 127, 127))
                    .font(font::Font {
                        family: font::Family::Name("Inter"),
                        weight: font::Weight::Semibold,
                        ..Default::default()
                    }),
            ]
            .align_x(Alignment::Center),
        };

        container(clock)
            .center(Length::Fill)
            .style(style_container)
            .into()
    }

    pub fn view_interaction(&self) -> iced::Element<Message> {
        InteractionTracker::new()
            .on_interaction(Message::Interact)
            .into()
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Tick(instant) => {
                self.now = chrono::Utc::now().with_timezone(&Europe::Berlin);
                let time = self.now.time();

                self.dim = self
                    .config
                    .dim
                    .iter()
                    .any(|dim| time >= dim.start && time <= dim.end);

                if self
                    .config
                    .exclude
                    .iter()
                    .any(|exclude| time >= exclude.start && time <= exclude.end)
                {
                    self.update(Message::Interact);
                }

                if instant - self.last_interaction > time::Duration::from_secs(self.config.timeout)
                {
                    self.state = State::Active;
                } else {
                    self.state = State::Inactive;
                }
            }
            Message::Interact => {
                log::debug!("Screensaver interaction");
                self.state = State::Inactive;
                self.last_interaction = time::Instant::now();
            }
        }
    }

    fn time(&self) -> String {
        self.now.format("%H:%M:%S").to_string()
    }

    fn date(&self) -> String {
        self.now
            .format_localized("%d. %B", chrono::Locale::de_DE)
            .to_string()
    }
}

fn style_container(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgb8(0, 0, 0))),
        text_color: Some(Color::from_rgb8(255, 255, 255)),
        ..Default::default()
    }
}
