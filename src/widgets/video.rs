use clapper::MediaItem;
use gtk::Box;
use gtk::Button;
use gtk::prelude::*;

use crate::config::Config;

pub struct Video {
    wrapper: Box,
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

        let button_wrapper = Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .css_classes(["buttons"])
            .build();

        for (idx, video) in config.videos.iter().enumerate() {
            let button = Button::builder()
                .label(&video.name)
                .hexpand(true)
                .build();

            let button_queue = queue.clone();
            button.connect_clicked(move |_| {
                button_queue.set_current_index(idx as u32);
            });

            button_wrapper.append(&button);
        }

        let wrapper = Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .css_classes(["video-player"])
            .build();

        wrapper.append(&video_player);
        wrapper.append(&button_wrapper);

        Video {
            wrapper,
        }
    }

    pub fn widget(&self) -> &Box {
        &self.wrapper
    }
}
