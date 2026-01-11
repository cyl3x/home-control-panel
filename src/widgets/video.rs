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
        let video_player = clapper_gtk::Video::new();
        video_player.set_visible(true);
        video_player.add_css_class("clapper");

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

        let button_wrapper = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        button_wrapper.add_css_class("buttons");

        let mut spinners = Vec::with_capacity(config.videos.len());
        for (idx, video) in config.videos.iter().enumerate() {
            let label = gtk::Label::new(Some(&video.name));
            label.set_hexpand(false);
            label.set_margin_vertical(2);

            let spinner = gtk::Spinner::new();
            spinner.set_visible(false);
            spinner.set_spinning(false);

            let spinner_wrapper = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            spinner_wrapper.set_expand(false);
            spinner_wrapper.set_align(gtk::Align::Center);
            spinner_wrapper.append(&spinner);

            let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            content.set_halign(gtk::Align::Center);
            content.set_hexpand(true);
            content.append(&spinner_wrapper);
            content.append(&label);

            let button = gtk::Button::new();
            button.set_hexpand(true);
            button.set_child(Some(&content));
            button.connect_clicked(move |_| {
                messaging::send_message(VideoMessage::VideoSelectIndex(Some(idx)));
            });

            button_wrapper.append(&button);
            spinners.push(spinner);
        }

        let wrapper = gtk::Box::new(gtk::Orientation::Vertical, 0);
        wrapper.add_css_class("video-player");
        wrapper.append(&video_player);
        wrapper.append(&button_wrapper);

        if !config.videos.is_empty() {
            messaging::send_message(VideoMessage::VideoSelectIndex(Some(0)));
        }

        glib::timeout_add_local_full(Duration::from_secs(5), Priority::DEFAULT_IDLE, || {
            messaging::send_message(VideoMessage::CheckVideoState(None));

            glib::ControlFlow::Continue
        });

        player.connect_state_notify(|player| {
            messaging::send_message(VideoMessage::CheckVideoState(Some(player.state())))
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
            VideoMessage::CheckVideoState(state) => match state.unwrap_or_else(|| self.player.state()) {
                PlayerState::Playing | PlayerState::Buffering => {
                    remove_source(self.reset_timeout.take());

                    for spinner in &self.spinners {
                        if spinner.is_visible() {
                            spinner.stop();
                            spinner.set_visible(false);
                        }
                    }
                }
                PlayerState::Stopped | PlayerState::Paused => {
                    remove_source(self.reset_timeout.take());

                    self.reset_timeout = Some(glib::timeout_add_seconds_local_once(3, move || {
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
