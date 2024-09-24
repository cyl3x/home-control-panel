use chrono::{DateTime, Utc};
use chrono_tz::Europe;
use gtk::prelude::*;
use relm4::prelude::*;

use crate::config;

#[derive(Debug)]
pub struct Widget {
  now: DateTime<Utc>,
  last_activity: DateTime<Utc>,
  config: config::Screensaver,
  visible: bool,
}

#[derive(Debug, Clone)]
pub enum Input {
  Tick(DateTime<Utc>),
  Reset,
}

#[derive(Debug, Clone)]
pub enum Output {}

#[relm4::component(pub)]
impl Component for Widget {
  type Init = config::Screensaver;
  type Input = Input;
  type Output = Output;
  type CommandOutput = ();

  view! {
    gtk::Box {
      add_css_class: "screensaver",
      set_orientation: gtk::Orientation::Vertical,
      inline_css: "font-size: 16px;",

      #[watch] set_visible: model.visible,
      add_controller = gtk::GestureClick {
        connect_pressed[sender] => move |_, _, _, _| {
          sender.input(Input::Reset);
        },
      },

      #[name(info)] gtk::Box {
        set_orientation: gtk::Orientation::Vertical,
        set_vexpand: true,
        set_hexpand: false,
        set_valign: gtk::Align::Center,
        set_halign: gtk::Align::Center,

        #[name(time)] gtk::Label {
          add_css_class: "screensaver-time",
          set_halign: gtk::Align::Center,
          set_valign: gtk::Align::Center,
          #[watch] set_text: &model.now
            .with_timezone(&Europe::Berlin)
            .format("%H:%M:%S")
            .to_string(),
        },

        #[name(date)] gtk::Label {
          add_css_class: "screensaver-date",
          set_halign: gtk::Align::Center,
          set_valign: gtk::Align::Center,
          #[watch] set_text: &model.now
            .with_timezone(&Europe::Berlin)
            .format_localized("%d. %B", chrono::Locale::de_DE)
            .to_string(),
        },
      },
    },
  }

  fn update_with_view(
    &mut self,
    widgets: &mut Self::Widgets,
    input: Self::Input,
    sender: ComponentSender<Self>,
    _root: &Self::Root,
  ) {
    match input {
      Input::Tick(now) => {
        self.now = now;
        let time = now.time();

        if self.config.exclude.iter().any(|exclude| time >= exclude.start && time <= exclude.end) {
          self.last_activity = now;
        }

        if self.config.dim.iter().any(|dim| time >= dim.start && time <= dim.end) {
          widgets.info.inline_css("font-size: 10px; opacity: 0.5;");
        } else {
          widgets.info.inline_css("font-size: 16px; opacity: 1;");
        }

        self.visible = (now - self.last_activity).num_seconds() > self.config.timeout as i64;
      }
      Input::Reset => {
        self.last_activity = Utc::now();
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
      now: Utc::now(),
      last_activity: Utc::now(),
      config,
      visible: false,
    };

    gtk::glib::timeout_add_seconds(1, gtk::glib::clone!(@strong sender => move || {
      sender.input(Input::Tick(chrono::Utc::now()));

      gtk::glib::ControlFlow::Continue
    }));

    let widgets = view_output!();

    ComponentParts { model, widgets }
  }
}
