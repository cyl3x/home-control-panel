use chrono::{DateTime, Utc};
use chrono_tz::Europe;
use gtk::glib::SourceId;
use gtk::prelude::*;
use relm4::prelude::*;

#[derive(Debug)]
pub struct Widget {
  timeout: Option<SourceId>,
}

#[derive(Debug, Clone)]
pub enum Input {
  Tick(DateTime<Utc>),
  Visible(bool),
  Reset,
}

#[derive(Debug, Clone)]
pub enum Output {}

#[relm4::component(pub)]
impl Component for Widget {
  type Init = ();
  type Input = Input;
  type Output = Output;
  type CommandOutput = ();

  view! {
    gtk::Box {
      add_css_class: "screensaver",
      set_orientation: gtk::Orientation::Vertical,
      add_controller = gtk::GestureClick {
        connect_pressed[sender] => move |_, _, _, _| {
          sender.input(Input::Visible(false));
        },
      },

      gtk::Box {
        set_orientation: gtk::Orientation::Vertical,
        set_vexpand: true,
        set_hexpand: false,
        set_valign: gtk::Align::Center,
        set_halign: gtk::Align::Center,

        #[name(time)] gtk::Label {
          add_css_class: "screensaver-time",
          set_halign: gtk::Align::Center,
          set_valign: gtk::Align::Center,
        },

        #[name(date)] gtk::Label {
          add_css_class: "screensaver-date",
          set_halign: gtk::Align::Center,
          set_valign: gtk::Align::Center,
        },
      },
    },
  }

  fn update_with_view(
    &mut self,
    widgets: &mut Self::Widgets,
    input: Self::Input,
    sender: ComponentSender<Self>,
    root: &Self::Root,
  ) {
    match input {
      Input::Tick(now) => {
        let local = now.with_timezone(&Europe::Berlin);

        widgets.time.set_text(&local.format("%H:%M:%S").to_string());
        widgets.date.set_text(&now.format_localized("%d. %B", chrono::Locale::de_DE).to_string());
      }
      Input::Visible(visible) => {
        if visible {
          let size = std::cmp::min(root.width() / 100, root.height() / 80);
          root.inline_css(&format!("font-size: {}px;", size));
        }

        root.set_visible(visible);
      }
      Input::Reset => {
        root.set_visible(false);

        if let Some(id) = self.timeout.take() {
          id.remove();
        }

        self.timeout = Some(gtk::glib::timeout_add_seconds(600, gtk::glib::clone!(@strong sender => move || {
          sender.input(Input::Visible(true));

          gtk::glib::ControlFlow::Continue
        })));
      }
    }
  }

  fn init(
    _: Self::Init,
    root: Self::Root,
    sender: ComponentSender<Self>,
  ) -> ComponentParts<Self> {
    let model = Self { timeout: None };

    gtk::glib::timeout_add_seconds(1, gtk::glib::clone!(@strong sender => move || {
      sender.input(Input::Tick(chrono::Utc::now()));

      gtk::glib::ControlFlow::Continue
    }));

    sender.input(Input::Reset);

    let widgets = view_output!();

    ComponentParts { model, widgets }
  }
}
