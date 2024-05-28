use gtk::{pango, prelude::*};
use relm4::prelude::*;

#[derive(Default, Debug)]
pub struct Widget {
  summary: String,
  description: String,
  tooltip: String,
}

pub type UpdateWidget = (String, String, String, Option<String>);

#[derive(Debug, Clone)]
pub enum Input {
  Update(UpdateWidget),
}

#[derive(Debug, Clone)]
pub enum Output {}

#[relm4::factory(pub)]
impl FactoryComponent for Widget {
  type Init = UpdateWidget;
  type Input = Input;
  type Output = Output;
  type ParentWidget = gtk::Box;
  type CommandOutput = ();

  view! {
    #[root]
    gtk::Box {
      add_css_class: "calendar-event-list-entry",
      set_orientation: gtk::Orientation::Horizontal,
      set_hexpand: true,
      set_halign: gtk::Align::Fill,
      set_valign: gtk::Align::Start,
      #[watch] set_tooltip: &self.tooltip,

      #[name(calendar_color_box)]
      gtk::Box {
        add_css_class: "calendar-event-list-entry-color-box",
        set_vexpand: true,
        set_size_request: (8, -1),
      },

      gtk::Box {
        set_orientation: gtk::Orientation::Vertical,
        set_hexpand: true,
        set_vexpand: true,

        gtk::Label {
          set_hexpand: true,
          set_halign: gtk::Align::Fill,
          set_valign: gtk::Align::Start,
          set_can_focus: false,
          set_ellipsize: pango::EllipsizeMode::End,
          #[watch] set_text: &self.summary,
        },

        gtk::Label {
          add_css_class: "dim-label",
          set_hexpand: true,
          set_halign: gtk::Align::Fill,
          set_valign: gtk::Align::Start,
          set_can_focus: false,
          set_ellipsize: pango::EllipsizeMode::End,
          #[watch] set_text: &self.description,
        },
      },
    },
  }

  fn update_with_view(&mut self, widgets: &mut Self::Widgets, input: Self::Input, sender: FactorySender<Self>) {
    match input {
      Input::Update((summary, description, tooltip, color)) => {
        self.summary = summary;
        self.description = description;
        self.tooltip = tooltip;

        if let Some(color) = color {
          widgets.calendar_color_box.inline_css(&format!("background-color: {};", color));
        } else {
          widgets.calendar_color_box.inline_css("");
        }
      }
    }

    self.update_view(widgets, sender);
  }

  fn init_model(init: Self::Init, _index: &Self::Index, sender: FactorySender<Self>) -> Self {
    sender.input(Input::Update(init));
    Self::default()
  }
}
