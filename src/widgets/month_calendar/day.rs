use std::collections::BTreeMap;

use chrono::{Datelike, NaiveDate, NaiveDateTime};
use gtk::prelude::*;
use relm4::{gtk, Component, ComponentParts, ComponentSender, RelmWidgetExt};
use uuid::Uuid;

use crate::calendar::Event;

#[derive(Debug, Default)]
pub struct Widget {
  day: NaiveDate,
  indicators: BTreeMap<Uuid, (u32, gtk::Box)>,
}

#[derive(Debug)]
pub enum Input {
  Tick(NaiveDateTime),
  SetDay(NaiveDate, bool),
  Add(Box<Event>),
  Reset,
  Select,
  Deselect,
  Clicked
}

#[derive(Debug)]
pub enum Output {
  Selected(NaiveDate),
}

#[relm4::component(pub)]
impl Component for Widget {
  type Init = ();
  type Input = Input;
  type Output = Output;
  type CommandOutput = ();

  view! {
    gtk::Overlay {
      #[name(background)]
      gtk::Box {},

      #[name(day)]
      add_overlay = &gtk::Box {
        add_css_class: "month-calendar-day",
        set_orientation: gtk::Orientation::Vertical,
        set_spacing: 2,

        add_controller = gtk::GestureClick {
          connect_pressed[sender] => move |controller, _, _, _| {
            if controller.current_button() == gtk::gdk::BUTTON_PRIMARY {
              sender.input(Input::Clicked);
            }
          },
        },

        #[name(label)]
        gtk::Label {
          add_css_class: "month-calendar-day-label",
          set_hexpand: true,
          #[watch] set_text: &model.day.day().to_string(),
        },

        #[name(indicators)]
        gtk::Box {
          set_orientation: gtk::Orientation::Horizontal,
          set_halign: gtk::Align::Center,
          set_spacing: 4,
        },
      },
    }
  }

  fn update_with_view(&mut self, widgets: &mut Self::Widgets, input: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
    match input {
      Input::SetDay(day, is_same_month) => {
        self.day = day;

        if is_same_month {
          widgets.day.remove_css_class("month-calendar-day-other-month");
        } else {
          widgets.day.add_css_class("month-calendar-day-other-month");
        }
      }
      Input::Add(event) => {
        if let Some((count, indicator)) = self.indicators.remove(&event.calendar_uid) {
          self.indicators.insert(event.calendar_uid, (count + 1, indicator));
        } else {
          let indicator = indicator_from_event(&event);
          widgets.indicators.append(&indicator);
          self.indicators.insert(event.calendar_uid, (1, indicator));
        }
      }
      Input::Reset => {
        let indicators = std::mem::take(&mut self.indicators);

        for (_, (_, indicator)) in indicators {
          indicator.unparent();
          drop(indicator);
        }

        sender.input(Input::Tick(chrono::Utc::now().naive_utc()));
      }
      Input::Select => {
        widgets.day.add_css_class("month-calendar-day-selected");
      }
      Input::Deselect => {
        widgets.day.remove_css_class("month-calendar-day-selected");
      }
      Input::Tick(now) => {
        if now.date() == self.day {
          widgets.background.add_css_class("month-calendar-day-today");
        } else {
          widgets.background.remove_css_class("month-calendar-day-today");
        }
      }
      Input::Clicked => {
        sender.output(Output::Selected(self.day)).unwrap();
      }
    }

    self.update_view(widgets, sender);
  }

  fn init(
    _: Self::Init,
    root: Self::Root,
    sender: ComponentSender<Self>,
  ) -> ComponentParts<Self> {
    let model = Self::default();

    let widgets = view_output!();

    root.set_measure_overlay(&widgets.day, true);

    ComponentParts { model, widgets }
  }
}

fn indicator_from_event(event: &Event) -> gtk::Box {
  let indicator = gtk::Box::default();
  indicator.set_size_request(12, 12);
  indicator.add_css_class("month-calendar-event-indicator");
  indicator.inline_css(&format!("background-color: {}", event.color()));

  indicator
}
