use std::time::Duration;

use clapper::MediaItem;
use clapper::Player;
use clapper::PlayerState;
use clapper::Queue;
use gtk::glib;
use gtk::glib::Priority;

use crate::config::Config;
use crate::messaging;
use crate::messaging::VideoMessage;
use crate::prelude::*;

pub struct Video {
    wrapper: gtk::Box,

    player: Player,
    queue: Queue,
    spinners: Vec<gtk::Spinner>,
}

impl Video {
    pub fn new(config: &Config) -> Self {
        let video_player = clapper_gtk::Video::builder()
            .visible(true)
            .css_classes(["clapper"])
            .build();

        let player = video_player.player().unwrap();
        player.set_audio_enabled(false);
        player.set_autoplay(false);
        player.set_subtitles_enabled(false);

        let queue = player.queue().unwrap();
        for video in &config.videos {
            let media_item = MediaItem::builder()
                .uri(video.url.as_str())
                .name(&video.name)
                .build();

            queue.add_item(&media_item);
        }

        let button_wrapper = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .css_classes(["buttons"])
            .build();

        let mut spinners = Vec::with_capacity(config.videos.len());
        for (idx, video) in config.videos.iter().enumerate() {
            let label = gtk::Label::builder()
                .label(&video.name)
                .hexpand(false)
                .build();

            let spinner = gtk::Spinner::builder()
                .visible(false)
                .height_request(4)
                .width_request(4)
                .build();

            let content = gtk::Box::builder()
                .orientation(gtk::Orientation::Horizontal)
                .halign(gtk::Align::Center)
                .hexpand(true)
                .spacing(4)
                .build();

            content.append(&label);
            content.append(&spinner);

            let button = gtk::Button::builder().child(&content).hexpand(true).build();

            button_wrapper.append(&button);

            button.connect_clicked(move |_| {
                messaging::send_message(VideoMessage::VideoSelectIndex(idx));
            });

            spinners.push(spinner);
        }

        let wrapper = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .css_classes(["video-player"])
            .build();

        wrapper.append(&video_player);
        wrapper.append(&button_wrapper);

        if !config.videos.is_empty() {
            messaging::send_message(VideoMessage::VideoSelectIndex(0));
        }

        glib::timeout_add_local_full(Duration::from_secs(5), Priority::DEFAULT_IDLE, || {
            messaging::send_message(VideoMessage::CheckVideoState);

            glib::ControlFlow::Continue
        });

        Self {
            wrapper,
            player,
            queue,
            spinners,
        }
    }

    pub const fn widget(&self) -> &gtk::Box {
        &self.wrapper
    }

    pub fn update(&mut self, message: VideoMessage) {
        match message {
            VideoMessage::CheckVideoState => match self.player.state() {
                PlayerState::Playing | PlayerState::Buffering => {
                    if self.spinners.first().is_some_and(|s| s.is_visible()) {
                        for spinner in &self.spinners {
                            spinner.stop();
                            spinner.set_visible(false);
                        }
                    }
                }
                PlayerState::Stopped | PlayerState::Paused => {
                    log::warn!("Video player: stopped, restarting current video");

                    let current_index = self.queue.current_index();
                    glib::timeout_add_seconds_once(1, move || {
                        messaging::send_message(VideoMessage::VideoSelectIndex(
                            current_index as usize,
                        ));
                    });
                }
                _ => (),
            },
            VideoMessage::VideoSelectIndex(clicked_idx) => {
                log::info!("Video player: selecting video index {}", clicked_idx);

                for (idx, spinner) in self.spinners.iter().enumerate() {
                    if idx == clicked_idx {
                        spinner.start();
                        spinner.set_visible(true);
                    } else {
                        spinner.stop();
                        spinner.set_visible(false);
                    }
                }

                self.player.seek(0.0);
                self.queue.set_current_index(clicked_idx as u32);
                self.player.play();
            }
        }
    }
}
