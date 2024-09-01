use gtk::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::{gtk, Component, ComponentParts, ComponentSender};

use crate::config;

mod button;

#[derive(Debug)]
pub struct Widget {
  media_items: Vec<clapper::MediaItem>,
  buttons: FactoryVecDeque<button::Widget>,
}

#[derive(Debug)]
pub enum Input {
  Clicked(usize),
}

#[relm4::component(pub)]
impl Component for Widget {
  type Init = Vec<config::Video>;
  type Input = Input;
  type Output = ();
  type CommandOutput = ();

  view! {
    gtk::Box {
      set_orientation: gtk::Orientation::Vertical,

      #[name(video)]
      clapper_gtk::Video {},

      append: model.buttons.widget(),
    }
  }

  fn update_with_view(&mut self, widgets: &mut Self::Widgets, input: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
    let queue = widgets.video.player().unwrap().queue().unwrap();

    match input {
      Input::Clicked(idx) => {
        queue.set_current_index(idx as u32);
      }
    }
  }

  fn init(
    videos: Self::Init,
    root: Self::Root,
    sender: ComponentSender<Self>,
  ) -> ComponentParts<Self> {
    let mut model = Self {
      media_items: videos.iter().map(create_media_item).collect(),
      buttons: FactoryVecDeque::builder()
        .launch(button::create_parent())
        .forward(sender.input_sender(), |output| {
          match output {
            button::Output::Clicked(idx) => Input::Clicked(idx),
          }
        }),
    };

    for video in &videos {
      model.buttons.guard().push_back(video.name.clone());
    }

    let widgets = view_output!();

    let player = widgets.video.player().unwrap();
    player.set_autoplay(true);
    player.set_audio_enabled(false);
    player.set_subtitles_enabled(false);

    let queue = player.queue().unwrap();
    for media_item in &model.media_items {
      queue.add_item(media_item);
    }

    ComponentParts { model, widgets }
  }
}

fn create_media_item(video: &config::Video) -> clapper::MediaItem {
  clapper::MediaItem::builder()
    .uri(video.url.as_str())
    .name(&video.name)
    .build()
}
