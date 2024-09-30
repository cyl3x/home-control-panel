use iced::widget::{button, row, text, Column};
use iced::{Length, Padding};
use iced_video_player::VideoPlayer;

use crate::config;

pub struct Video {
    videos: Vec<config::Video>,
    video: Option<iced_video_player::Video>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SetVideo(usize),
}

impl Video {
    pub fn new(videos: Vec<config::Video>) -> Self {
        let mut video = Self {
            videos,
            video: None,
        };

        if !video.videos.is_empty() {
            video.update(Message::SetVideo(0));
        }

        video
    }

    pub fn view(&self) -> iced::Element<Message> {
        if self.videos.is_empty() {
            return text("No videos").into();
        }

        let mut column = Column::new();

        if let Some(video) = &self.video {
            column = column.push(VideoPlayer::new(video));
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
        match message {
            Message::SetVideo(idx) => self.video = rtsp_video(&self.videos[idx].url),
        }
    }
}

fn rtsp_video(url: &url::Url) -> Option<iced_video_player::Video> {
    let pipeline = iced_video_player::Video::from_pipeline(
        format!("uridecodebin uri={} ! videoconvert ! videoscale ! videorate ! appsink name=iced_video caps=video/x-raw,format=RGBA,pixel-aspect-ratio=1/1", url),
        Some(true),
    );

    match pipeline {
        Ok(video) => Some(video),
        Err(err) => {
            log::error!("Error starting video: {:?}", err);
            None
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
