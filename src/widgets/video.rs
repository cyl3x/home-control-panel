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

    reset_timeout: Option<glib::SourceId>,
}

impl Video {
    pub fn new(config: &Config) -> Self {
        let video_player = clapper_gtk::Video::builder()
            .visible(true)
            .css_classes(["clapper"])
            .build();

        let player = video_player.player().unwrap();
        player.set_audio_enabled(false);
        player.set_autoplay(true);
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
                messaging::send_message(VideoMessage::VideoSelectIndex(Some(idx)));
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
            messaging::send_message(VideoMessage::VideoSelectIndex(Some(0)));
        }

        glib::timeout_add_local_full(Duration::from_secs(3), Priority::DEFAULT_IDLE, || {
            messaging::send_message(VideoMessage::CheckVideoState);

            glib::ControlFlow::Continue
        });

        Self {
            wrapper,
            player,
            queue,
            spinners,
            reset_timeout: None,
        }
    }

    pub const fn widget(&self) -> &gtk::Box {
        &self.wrapper
    }

    pub fn update(&mut self, message: VideoMessage) {
        match message {
            VideoMessage::CheckVideoState => match self.player.state() {
                PlayerState::Playing | PlayerState::Buffering => {
                    for spinner in &self.spinners {
                        if spinner.is_visible() {
                            spinner.stop();
                            spinner.set_visible(false);
                        }
                    }
                }
                PlayerState::Stopped | PlayerState::Paused => {
                    self.reset_timeout = Some(glib::timeout_add_seconds_local_once(1, move || {
                        messaging::send_message(VideoMessage::VideoSelectIndex(None));
                    }));
                }
                _ => (),
            },
            VideoMessage::VideoSelectIndex(clicked_idx) => {
                remove_source(self.reset_timeout.take());

                let clicked_idx = clicked_idx.unwrap_or_else(|| self.queue.current_index() as usize);
                log::info!("Video player: selecting video \"{}\"", self.item_name(clicked_idx));

                for (idx, spinner) in self.spinners.iter().enumerate() {
                    if idx == clicked_idx {
                        spinner.start();
                        spinner.set_visible(true);
                    } else {
                        spinner.stop();
                        spinner.set_visible(false);
                    }
                }

                self.queue.select_item(None);
                self.queue.set_current_index(clicked_idx as u32);
                self.player.play();
            }
        }
    }

    fn item_name(&self, idx: usize) -> String {
        self.queue
            .item(idx as u32)
            .and_then(|item| item.property_value("name").get::<String>().ok())
            .unwrap_or_else(|| format!("index {}", idx))
    }
}
