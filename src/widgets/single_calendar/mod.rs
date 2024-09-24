use chrono::{NaiveDate, NaiveDateTime};
use gtk::prelude::*;
use relm4::prelude::*;

use crate::calendar::Event;

#[derive(Debug)]
pub struct Widget {
  date: NaiveDate,
  event: Option<Event>,
}

#[derive(Debug, Clone)]
pub enum Input {
  Tick(NaiveDateTime),
  Add(Box<Event>),
  Reset,
}

#[derive(Debug, Clone)]
pub enum Output {
  RequestEvents(NaiveDate),
}

#[relm4::component(pub)]
impl Component for Widget {
  type Init = ();
  type Input = Input;
  type Output = Output;
  type CommandOutput = ();

  view! {
    gtk::Box {
      add_css_class: "single-calendar",
      set_orientation: gtk::Orientation::Horizontal,
      set_valign: gtk::Align::Center,
      set_halign: gtk::Align::Start,
      set_hexpand: false,
      set_vexpand: false,
      set_margin_horizontal: 16,
      set_margin_vertical: 16,
      set_spacing: 8,
      #[watch] set_tooltip: &model.event.as_ref().map_or_else(String::new, |e| e.tooltip()),

      gtk::Box {
        set_vexpand: false,
        set_hexpand: false,
        set_valign: gtk::Align::Center,
        set_size_request: (20, 20),
        add_css_class: "single-calendar-indicator",
        #[watch] inline_css: &format!("background-color: {}", model.event.as_ref().map_or("", |e| e.color())),
      },

      gtk::Label {
        add_css_class: "single-calendar-date",
        set_vexpand: false,
        set_hexpand: false,
        #[watch] set_text: &model.event.as_ref().map_or_else(String::new, |e| e.start_tz().format("(%d.%m)").to_string()),
      },

      gtk::Label {
        add_css_class: "single-calendar-summary",
        set_vexpand: false,
        set_hexpand: true,
        #[watch] set_text: &model.event.as_ref().map_or("", |e| &e.summary),
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
      Input::Add(event) => {
        if self.event.is_none() {
          self.event = Some(*event);
          root.set_visible(true);
        }
      }
      Input::Reset => {
        self.event = None;
        root.set_visible(false);
        sender.output(Output::RequestEvents(self.date)).unwrap();
      }
      Input::Tick(now) => {
        if now.date() != self.date {
          self.date = now.date();
          sender.input(Input::Reset);
        }
      }
    }

    self.update_view(widgets, sender);
  }

  fn init(
    _: Self::Init,
    root: Self::Root,
    _sender: ComponentSender<Self>,
  ) -> ComponentParts<Self> {
    let model = Self {
      date: chrono::Utc::now().date_naive(),
      event: None,
    };

    let widgets = view_output!();

    ComponentParts { model, widgets }
  }
}
