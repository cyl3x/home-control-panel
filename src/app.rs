use gtk::prelude::*;
use relm4::prelude::*;
use relm4::{Component, ComponentController, ComponentParts, ComponentSender};

use crate::components::calendar;
use crate::components::video::VideoComponent;
use crate::config::Config;

pub struct App<> {
  calendar: Controller<calendar::Widget>,
  config: Config,
}

#[derive(Debug)]
pub enum Input {}

#[relm4::component(pub)]
impl Component for App {
  type Init = Config;
  type Input = Input;
  type Output = ();
  type CommandOutput = ();

  view! {
    gtk::Window {
      set_default_size: (600, 300),

      #[name(calendar_and_cams_paned)]
      gtk::Paned {
        set_orientation: gtk::Orientation::Horizontal,
        set_vexpand: true,
        set_hexpand: true,
        set_wide_handle: true,

        set_start_child: Some(model.calendar.widget().widget_ref()),

        #[wrap(Some)]
        #[name(cams_box)]
        set_end_child = &gtk::Box {
          inline_css: "background-color: #000000;",

          set_orientation: gtk::Orientation::Vertical,
          set_vexpand: true,
          set_hexpand: true,
          set_size_request: (100, -1),
        },
      }
    }
  }

  fn update(&mut self, input: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
    match input {
    }
  }

  fn init(
    config: Self::Init,
    root: Self::Root,
    _sender: ComponentSender<Self>,
  ) -> ComponentParts<Self> {
    let provider = crate::calendar::init(&config);

    let calendar = calendar::Widget::builder().launch(provider).detach();

    let model = Self { calendar, config };

    let widgets = view_output!();

    if let Some(videos) = &model.config.videos {
      for url in videos {
        let rtsp = VideoComponent::builder().launch(url.clone()).detach();
        widgets.cams_box.append(rtsp.widget());
      }
    }

    ComponentParts { model, widgets }
  }
}

impl App {}
