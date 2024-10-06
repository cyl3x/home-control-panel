use std::time::Duration;

use iced::widget::{button, row, text, Column};
use iced::{Length, Padding};
use iced_video_player::VideoPlayer;

use crate::config;

pub struct Video {
    videos: Vec<config::Video>,
    video: Option<iced_video_player::Video>,
    is_error: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    SetVideo(usize),
    RestartVideo,
}

impl Video {
    pub fn new(videos: Vec<config::Video>) -> Self {
        let mut video = Self {
            videos,
            video: None,
            is_error: false,
        };

        if !video.videos.is_empty() {
            video.update(Message::SetVideo(0));
        }

        video
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        match &self.is_error {
            true => iced::time::every(Duration::from_secs(300)).map(|_| Message::RestartVideo),
            false => iced::Subscription::none(),
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        if self.videos.is_empty() {
            return text("No videos").into();
        }

        let mut column = Column::new();

        if let Some(video) = &self.video {
            column = column.push(VideoPlayer::new(video).on_error(|err| {
                log::error!("Video error, restarting: {:?}", err);
                Message::RestartVideo
            }));
        }

        let buttons = self.videos.iter().enumerate().map(|(idx, video)| {
            button(text(&video.name).center())
                .on_press(Message::SetVideo(idx))
                .width(Length::Fill)
                .height(36)
                .style(style_button)
                .into()
        });

        let button_row = row(buttons)
            .spacing(16)
            .padding(Padding::ZERO.right(16.0))
            .width(Length::Fill)
            .height(Length::Shrink);

        column.push(button_row).spacing(16).into()
    }

    pub fn update(&mut self, message: Message) {
        self.is_error = false;

        match message {
            Message::SetVideo(idx) => {
                let pipeline = iced_video_player::Video::from_pipeline(
                    format!("uridecodebin uri={} ! videoconvert ! videoscale ! videorate ! appsink name=iced_video caps=video/x-raw,format=NV12,pixel-aspect-ratio=1/1", &self.videos[idx].url),
                    Some(true),
                );

                if let Some(video) = std::mem::replace(&mut self.video, None) {
                    std::mem::drop(video);
                }

                self.video = match pipeline {
                    Ok(video) => Some(video),
                    Err(err) => {
                        self.is_error = true;
                        log::error!("Error starting video: {:?}", err);
                        None
                    }
                };
            }
            Message::RestartVideo => {
                let Some(video) = &mut self.video else { return };

                let Err(err) = video.restart_stream() else {
                    return;
                };

                self.is_error = true;

                log::error!("Error restarting video: {:?}", err);
            }
        }
    }
}

pub fn style_button(theme: &iced::Theme, _: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    button::Style {
        text_color: palette.primary.strong.text,
        background: Some(palette.primary.strong.color.into()),
        border: iced::Border::default().rounded(3),
        ..Default::default()
    }
}
