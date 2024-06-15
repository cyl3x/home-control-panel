use chrono::NaiveDateTime;
use gtk::{pango, prelude::*};
use relm4::prelude::*;
use uuid::Uuid;

use crate::calendar::Event;

#[derive(Debug)]
pub struct Widget {
  event: Event,
  now: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub enum Input {
  Tick(NaiveDateTime),
}

#[derive(Debug, Clone)]
pub enum Output {}

#[relm4::factory(pub)]
impl FactoryComponent for Widget {
  type Init = Event;
  type Input = Input;
  type Output = Output;
  type ParentWidget = gtk::ListBox;
  type CommandOutput = ();
  type Index = Uuid;

  view! {
    gtk::Box {
      add_css_class: "days-calendar-day-entry",
      set_orientation: gtk::Orientation::Horizontal,
      set_hexpand: true,
      set_spacing: 4,
      #[watch] set_tooltip: &self.event.tooltip(),
      #[watch] set_widget_name: &self.event.start.to_string(),

      gtk::Box {
        add_css_class: "days-calendar-day-entry-indicator",
        #[watch] inline_css: &format!("background-color: {};", self.event.color()),
        set_vexpand: true,
        set_size_request: (8, -1),
      },

      gtk::Box {
        set_orientation: gtk::Orientation::Vertical,
        set_vexpand: true,
        set_hexpand: true,

        gtk::Label {
          set_halign: gtk::Align::Start,
          set_can_focus: false,
          set_ellipsize: pango::EllipsizeMode::End,
          #[watch] set_text: &self.event.summary,
        },

        gtk::Label {
          add_css_class: "dim-label",
          set_halign: gtk::Align::Start,
          set_can_focus: false,
          set_ellipsize: pango::EllipsizeMode::End,
          #[watch] set_text: self.event.description(),
        },
      },

      gtk::Box {
        set_vexpand: true,
        set_hexpand: true,

        gtk::Label {
          add_css_class: "days-calendar-day-entry-time-delta",
          set_vexpand: true,
          set_hexpand: true,
          set_halign: gtk::Align::End,
          #[watch] set_text: &self.event.delta_text(self.now),
        }
      },
    },
  }

  fn init_model(event: Self::Init, _index: &Self::Index, _sender: FactorySender<Self>) -> Self {
    Self { event, now: chrono::Utc::now().naive_utc() }
  }

  fn update(&mut self, input: Self::Input, _sender: FactorySender<Self>) {
    match input {
      Input::Tick(now) => self.now = now,
    }
  }
}
