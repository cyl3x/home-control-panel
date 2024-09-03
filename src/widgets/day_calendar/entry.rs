use chrono::NaiveDateTime;
use gtk::{pango, prelude::*, ListBoxRow};
use relm4::prelude::*;

use crate::calendar::event_uuid::EventUuid;
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
  type Index = EventUuid;

  view! {
    gtk::Box {
      add_css_class: "day-calendar-entry",
      set_orientation: gtk::Orientation::Horizontal,
      set_hexpand: true,
      set_spacing: 8,
      set_margin_top: 4,
      set_margin_bottom: 4,
      #[watch] set_tooltip: &self.event.tooltip(),
      #[watch] set_widget_name: &self.event.start.to_string(),

      gtk::Box {
        add_css_class: "day-calendar-entry-indicator",
        #[watch] inline_css: &format!("background-color: {};", self.event.color()),
        set_vexpand: true,
        set_size_request: (8, -1),
      },

      gtk::Box {
        set_orientation: gtk::Orientation::Vertical,
        set_vexpand: true,
        set_hexpand: true,

        gtk::Label {
          add_css_class: "day-calendar-entry-summary",
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
          add_css_class: "day-calendar-entry-time",
          set_vexpand: true,
          set_hexpand: true,
          set_halign: gtk::Align::End,
          #[watch] set_text: &self.event.start.time().format("%H:%M").to_string(),
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

pub fn create_parent() -> gtk::ListBox {
  let list = gtk::ListBox::builder()
    .halign(gtk::Align::Fill)
    .valign(gtk::Align::Start)
    .hexpand(true)
    .selection_mode(gtk::SelectionMode::None)
    .css_classes(["day-calendar"])
    .build();

  list.set_sort_func(sort_func);

  list
}

fn sort_func(a: &ListBoxRow, b: &ListBoxRow) -> gtk::Ordering {
  let a = a.child().map(|a| a.widget_name()).unwrap();
  let b = b.child().map(|a| a.widget_name()).unwrap();

  match a.cmp(&b) {
    std::cmp::Ordering::Less => gtk::Ordering::Smaller,
    std::cmp::Ordering::Equal => gtk::Ordering::Equal,
    std::cmp::Ordering::Greater => gtk::Ordering::Larger,
  }
}
