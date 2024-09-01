use gtk::prelude::*;
use relm4::prelude::*;
use relm4::{Component, ComponentParts, ComponentSender};

use crate::calendar::caldav;
use crate::config::Config;
use crate::widgets::view;

pub struct App<> {
  view: Controller<view::Widget>
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
      add_css_class: "window",
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

        gtk::Box {
          append: model.view.widget(),
        }
      }
    }
  }

  fn update_with_view(&mut self, widgets: &mut Self::Widgets, input: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
    match input {
      Input::CalDavError(err) => {
        // let msg = &format!("{err:#?}");
        // widgets.status_bar.context_id(msg);
        // widgets.status_bar.push(0, msg);
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
    let model = Self {
      view: view::Widget::builder().launch(config).forward(sender.input_sender(), |output| match output {
        view::Output::CalDavError(err) => Self::Input::CalDavError(err),
      }),
    };

    let widgets = view_output!();

    ComponentParts { model, widgets }
  }
}

impl App {}
