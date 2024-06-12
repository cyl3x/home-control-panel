use gtk::prelude::*;
use relm4::{gtk, Component, ComponentParts, ComponentSender, RelmWidgetExt};

use crate::config;

pub struct Widget {
  media_items: Vec<clapper::MediaItem>,
  drop_down: gtk::DropDown,
}

#[derive(Debug)]
pub enum Input {
  Previous,
  Next,
}

#[relm4::component(pub)]
impl Component for Widget {
  type Init = Vec<config::Video>;
  type Input = Input;
  type Output = ();
  type CommandOutput = ();

  view! {
    clapper_gtk::Video {
      add_fading_overlay = &gtk::Box {
        set_orientation: gtk::Orientation::Horizontal,
        set_hexpand: true,
        set_valign: gtk::Align::Start,
        set_halign: gtk::Align::Fill,
        set_margin_all: 16,
        set_spacing: 8,

        gtk::Button {
          set_css_classes: &["osd"],
          set_icon_name: "pan-start-symbolic",
          set_size_request: (52, 52),
          set_halign: gtk::Align::Start,

          connect_clicked => Input::Previous,
        },

        gtk::Box {
          set_css_classes: &["osd"],
          set_hexpand: true,
          set_halign: gtk::Align::Fill,

          append: &model.drop_down,
        },

        gtk::Button {
          set_css_classes: &["osd"],
          set_icon_name: "pan-end-symbolic",
          set_size_request: (52, 52),
          set_halign: gtk::Align::Start,

          connect_clicked => Input::Next,
        },
      }
    }
  }

  fn update_with_view(&mut self, _widgets: &mut Self::Widgets, input: Self::Input, _sender: ComponentSender<Self>, root: &Self::Root) {
    let queue = root.player().unwrap().queue().unwrap();

    match input {
      Input::Previous => {
        queue.select_previous_item();
      }
      Input::Next => {
        queue.select_next_item();
      }
    }
  }

  fn init(
    videos: Self::Init,
    root: Self::Root,
    sender: ComponentSender<Self>,
  ) -> ComponentParts<Self> {
    let drop_down = gtk::DropDown::from_strings(&create_names(&videos));

    let model = Self {
      media_items: create_media_items(&videos),
      drop_down,
    };

    let widgets = view_output!();

    let player = root.player().unwrap();
    player.set_autoplay(true);
    player.set_audio_enabled(false);
    player.set_subtitles_enabled(false);

    let queue = player.queue().unwrap();
    for media_item in &model.media_items {
      queue.add_item(media_item);
    }

    model.drop_down.set_hexpand(true);
    model.drop_down.set_halign(gtk::Align::Fill);
    model.drop_down.set_opacity(1.0);
    model.drop_down.bind_property("selected", &queue, "current_index").bidirectional().build();

    ComponentParts { model, widgets }
  }
}

fn create_media_items(videos: &[config::Video]) -> Vec<clapper::MediaItem> {
  videos
    .iter()
    .map(|video| clapper::MediaItem::builder()
      .uri(video.url.as_str())
      .name(&video.name)
      .build()
    )
    .collect()
}

fn create_names(videos: &[config::Video]) -> Vec<&str> {
  videos.iter().map(|video| video.name.as_str()).collect()
}
