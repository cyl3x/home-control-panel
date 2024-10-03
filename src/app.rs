use iced::widget::{container, pane_grid, stack};
use iced::Subscription;

use crate::config::Config;
use crate::views::{self, screensaver};

pub struct App {
    video: views::Video,
    calendar: views::Calendar,
    screensaver: views::Screensaver,
    panes: pane_grid::State<PaneState>,
}

enum PaneState {
    Video,
    Calendar,
}

#[derive(Debug)]
pub enum Message {
    Calendar(views::calendars::Message),
    Video(views::video::Message),
    Screensaver(views::screensaver::Message),
    PaneResized(pane_grid::ResizeEvent),
}

impl App {
    pub fn new(config: Config) -> (Self, iced::Task<Message>) {
        let (mut state, pane) = pane_grid::State::new(PaneState::Calendar);

        state.split(pane_grid::Axis::Vertical, pane, PaneState::Video);

        let (calendar, task) = views::Calendar::new(config.ical, config.calendar);

        (
            Self {
                video: views::Video::new(config.videos),
                calendar,
                screensaver: views::Screensaver::new(config.screensaver),
                panes: state,
            },
            task.map(Message::Calendar),
        )
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            self.calendar.subscription().map(Message::Calendar),
            self.screensaver.subscription().map(Message::Screensaver),
            self.video.subscription().map(Message::Video),
        ])
    }

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::Calendar(calendar_message) => self
                .calendar
                .update(calendar_message)
                .map(Message::Calendar),
            Message::Video(video_message) => self.video.update(video_message).map(Message::Video),
            Message::PaneResized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(split, ratio);

                iced::Task::none()
            }
            Message::Screensaver(screensaver_message) => {
                let before = self.screensaver.state;

                self.screensaver.update(screensaver_message);

                match (before, self.screensaver.state) {
                    (screensaver::State::Inactive, screensaver::State::Active) => {
                        log::info!("Screensaver activated");
                    }
                    (screensaver::State::Active, screensaver::State::Inactive) => {
                        log::info!("Screensaver deactivated");
                    }
                    _ => {}
                }

                iced::Task::none()
            }
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        stack![
            match self.screensaver.state {
                screensaver::State::Inactive => {
                    pane_grid(&self.panes, |_, state, _| {
                        pane_grid::Content::new(match state {
                            PaneState::Video => container(self.video.view().map(Message::Video)),
                            PaneState::Calendar => {
                                container(self.calendar.view().map(Message::Calendar))
                            }
                        })
                    })
                    .spacing(12)
                    .on_resize(12, Message::PaneResized)
                    .into()
                }
                screensaver::State::Active =>
                    self.screensaver.view_active().map(Message::Screensaver),
            },
            self.screensaver
                .view_interaction()
                .map(Message::Screensaver),
        ]
        .into()
    }
}
