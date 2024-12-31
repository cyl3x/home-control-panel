use std::time::Duration;
use glib::object::{Cast, ObjectExt};
use gstreamer::prelude::*;
use gstreamer as gst;
use gstreamer_app as gst_app;

use iced::widget::{button, row, text, Column};
use iced::{Length, Padding};
use iced_video_player::VideoPlayer;

use crate::config;

pub struct Video {
    videos: Vec<config::Video>,
    video: Option<iced_video_player::Video>,
    playing: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SetVideo(usize),
    RestartVideo,
    CheckVideo,
}

impl Video {
    pub fn new(videos: Vec<config::Video>) -> Self {
        let mut video = Self {
            videos,
            video: None,
            playing: None,
        };

        if !video.videos.is_empty() {
            video.update(Message::SetVideo(0));
        }

        video
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(Duration::from_secs(60)).map(|_| Message::CheckVideo)
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
        match message {
            Message::SetVideo(idx) => {
                self.playing = Some(idx);

                let pipeline = from_pipeline(&self.videos[idx].url);

                if let Some(video) = self.video.take() {
                    std::mem::drop(video);
                }

                self.video = match pipeline {
                    Ok(video) => Some(video),
                    Err(err) => {
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

                log::error!("Error restarting video: {:?}", err);
            }
            Message::CheckVideo => {
                if let Some(video) = &mut self.video {
                    let pipeline = video.pipeline();
                    let state = pipeline.current_state();

                    if video.eos() || ![gst::State::Ready, gst::State::Playing].contains(&state) {
                        log::warn!("Video stopped, restarting: eos={} | state={:?}", video.eos(), state);
                        self.update(Message::RestartVideo);
                    };
                } else if let Some(idx) = self.playing {
                    self.update(Message::SetVideo(idx))
                }
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

fn from_pipeline(uri: &url::Url) -> Result<iced_video_player::Video, iced_video_player::Error> {
    gst::init()?;

    let pipeline = format!("playbin uri=\"{}\" video-sink=\"videoconvert ! videoscale ! videorate ! appsink name=iced_video drop=true caps=video/x-raw,format=NV12,pixel-aspect-ratio=1/1,framerate=24/1\"", uri.as_str());
    let pipeline = gst::parse::launch(pipeline.as_ref())?
        .downcast::<gst::Pipeline>()
        .map_err(|_| iced_video_player::Error::Cast)?;

    let video_sink: gst::Element = pipeline.property("video-sink");
    let pad = video_sink.pads().first().cloned().unwrap();
    let pad = pad.dynamic_cast::<gst::GhostPad>().unwrap();
    let bin = pad
        .parent_element()
        .unwrap()
        .downcast::<gst::Bin>()
        .unwrap();
    let app_sink = bin.by_name("iced_video").unwrap();
    let app_sink = app_sink.downcast::<gst_app::AppSink>().unwrap();

    iced_video_player::Video::from_gst_pipeline(pipeline, app_sink, None)
}
