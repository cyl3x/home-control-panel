use glib::object::{Cast, ObjectExt};
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use std::time::{Duration, Instant};

use iced::widget::{button, row, text, Column};
use iced::{Length, Padding};
use iced_video_player::VideoPlayer;

use crate::config;

pub struct Video {
    videos: Vec<config::Video>,
    video: Option<iced_video_player::Video>,
    playing: Option<usize>,

    restart_trys: u8,
    restart_trigger: Option<Instant>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SetVideo(usize),
    ResetVideo,
    RestartVideo,
    CheckVideo,
}

impl Video {
    pub fn new(videos: Vec<config::Video>) -> Self {
        let mut video = Self {
            videos,
            video: None,
            playing: None,

            restart_trys: 0,
            restart_trigger: None,
        };

        if !video.videos.is_empty() {
            video.update(Message::SetVideo(0));
        }

        video
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::Subscription::batch([
            iced::time::every(Duration::from_secs(60)).map(|_| Message::CheckVideo),
            self.restart_trigger
                .map_or_else(iced::Subscription::none, |instant| {
                    let trigger_in = instant + Duration::from_secs(2);
                    let duration = trigger_in - Instant::now();

                    if duration > Duration::ZERO {
                        iced::time::every(duration).map(|_| Message::RestartVideo)
                    } else {
                        iced::time::every(Duration::from_secs(1)).map(|_| Message::RestartVideo)
                    }
                }),
        ])
    }

    pub fn view(&self) -> iced::Element<Message> {
        if self.videos.is_empty() {
            return text("No videos").into();
        }

        let mut column = Column::new();

        if let Some(video) = &self.video {
            let player = VideoPlayer::new(video)
                .on_error(|err| {
                    log::error!("Error while playing video: {:?}", err);
                    Message::ResetVideo
                })
                .on_end_of_stream(Message::RestartVideo)
                .width(Length::Fill);

            column = column.push(player);
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
                self.restart_trigger = None;
                self.restart_trys = 0;

                let pipeline = from_pipeline(&self.videos[idx].url);

                if let Some(video) = self.video.take() {
                    std::mem::drop(video);
                }

                self.video = match pipeline {
                    Ok(video) => {
                        log::info!("Set playing video to: {}", idx);
                        Some(video)
                    }
                    Err(err) => {
                        log::error!("Error starting video: {:?}", err);
                        None
                    }
                };
            }
            Message::ResetVideo => self.update(
                self.playing
                    .map_or(Message::RestartVideo, Message::SetVideo),
            ),
            Message::RestartVideo => {
                let Some(video) = &mut self.video else { return };

                if self.restart_trigger.is_none() {
                    return;
                }

                match video.restart_stream() {
                    Ok(_) => {
                        self.restart_trigger = None;
                        self.restart_trys = 0;
                    }
                    Err(err) => {
                        // delay restarts 2sec
                        self.restart_trigger = Some(Instant::now());
                        self.restart_trys += 1;

                        log::error!("Error restarting video ({}): {:?}", self.restart_trys, err);
                    }
                }
            }
            Message::CheckVideo => {
                if let Some(video) = &mut self.video {
                    let pipeline = video.pipeline();
                    let state = pipeline.current_state();

                    if video.eos() || ![gst::State::Ready, gst::State::Playing].contains(&state) {
                        log::warn!(
                            "Checking video failed, restarting vidoe: eos={} | state={:?}",
                            video.eos(),
                            state
                        );
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

    let pipeline = format!("playbin uri=\"{}\" video-sink=\"videorate ! videoscale ! appsink name=iced_video drop=true caps=video/x-raw,format=NV12,pixel-aspect-ratio=1/1,framerate=25/1,width=[1,2048]\"", uri.as_str());
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
