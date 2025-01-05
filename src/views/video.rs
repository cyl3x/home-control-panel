use iced::widget::column;

use crate::config;
use crate::widgets::video;

pub struct Video {
    pub video: video::Video,
}

#[derive(Debug, Clone)]
pub enum Message {
    VideoMessage(video::Message),
    CheckVideo,
}

impl Video {
    pub fn new(videos: Vec<config::Video>) -> Self {
        Self {
            video: video::Video::new(videos),
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        self.video.subscription().map(Message::VideoMessage)
    }

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::VideoMessage(video_message) => self.video.update(video_message),
            Message::CheckVideo => self.video.update(video::Message::CheckVideo),
        }

        iced::Task::none()
    }

    pub fn view(&self) -> iced::Element<Message> {
        column![self.video.view().map(Message::VideoMessage)].into()
    }
}
