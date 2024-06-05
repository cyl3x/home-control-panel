use gtk::prelude::*;
use relm4::prelude::*;
use relm4::{Component, ComponentController, ComponentParts, ComponentSender};

use crate::calendar::caldav::Credentials;
use crate::calendar::caldav;
use crate::components::calendar;
use crate::components::video;
use crate::config::Config;

pub struct App<> {
  calendar: Controller<calendar::Widget>,
  config: Config,
}

#[derive(Debug)]
pub enum Input {
  CalDavError(caldav::Error),
}

#[relm4::component(pub)]
impl Component for App {
  type Init = Config;
  type Input = Input;
  type Output = ();
  type CommandOutput = ();

  view! {
    gtk::Window {
      set_default_size: (600, 300),

      #[name(window_overlay)]
      gtk::Overlay {

        // #[name(notification_overlay)]
        // add_overlay = &gtk::Box {
        //   add_css_class: "notification-overlay-box",
        //   add_css_class: "error",

        //   set_orientation: gtk::Orientation::Vertical,
        //   set_vexpand: true,
        //   set_hexpand: true,
        //   set_valign: gtk::Align::End,
        //   set_halign: gtk::Align::End,
        //   set_margin_bottom: 16,

        //   #[name(loading_label)]
        //   gtk::Label {
        //     set_text: "Loading...",
        //     set_halign: gtk::Align::Start,
        //     set_valign: gtk::Align::Start,
        //   }
        // },

        #[name(status_bar)]
        add_overlay = &gtk::Statusbar {
          set_hexpand: true,
          set_vexpand: true,
          set_valign: gtk::Align::End,
          set_halign: gtk::Align::End,
        },

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
  }

  fn update_with_view(&mut self, widgets: &mut Self::Widgets, input: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
    match input {
      Input::CalDavError(err) => {
        let msg = &format!("{:#?}", err);
        widgets.status_bar.context_id(msg);
        widgets.status_bar.push(0, msg);
        log::error!("CalDav error: {:?}", err);
      }
    }

    self.update_view(widgets, sender);
  }

  fn init(
    config: Self::Init,
    root: Self::Root,
    sender: ComponentSender<Self>,
  ) -> ComponentParts<Self> {
    let calendar = calendar::Widget::builder()
      .launch((Credentials::from(&config), config.ical.url.clone()))
      .forward(
        sender.input_sender(),
        |output| match output {
          calendar::Output::CalDavError(err) => Input::CalDavError(err),
        }
      );

    let model = Self { calendar, config };

    let widgets = view_output!();

    if let Some(videos) = &model.config.videos {
      for url in videos {
        let rtsp = video::Widget::builder().launch(url.clone()).detach();
        widgets.cams_box.append(rtsp.widget());
      }
    }

    ComponentParts { model, widgets }
  }
}

impl App {}
