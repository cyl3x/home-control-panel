use chrono::{NaiveDate, NaiveDateTime};
use gtk::{pango, prelude::*};
use relm4::prelude::*;

use crate::calendar::event_uuid::EventUuid;
use crate::calendar::Event;

#[derive(Debug)]
pub struct Widget {
  date: NaiveDate,
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
  type Init = (NaiveDate, Event);
  type Input = Input;
  type Output = Output;
  type ParentWidget = gtk::ListBox;
  type CommandOutput = ();
  type Index = EventUuid;

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
          #[watch] set_text: &delta_text(&self.event, self.now, self.date),
        }
      },
    },
  }

  fn init_model((date, event): Self::Init, _index: &Self::Index, _sender: FactorySender<Self>) -> Self {
    Self { date, event, now: chrono::Utc::now().naive_utc() }
  }

  fn update(&mut self, input: Self::Input, _sender: FactorySender<Self>) {
    match input {
      Input::Tick(now) => self.now = now,
    }
  }
}

pub fn delta_text(event: &Event, date: NaiveDateTime, for_date: NaiveDate) -> String {
  let mut start = event.start;
  let end = event.end;

  if event.is_between_dates(for_date, for_date) {
    start = for_date.and_hms(0, 0, 0);
  }

  if date <= end && date >= start {
    return "jetzt".to_string();
  }

  let delta = if date > end {
    end - date
  } else {
    start - date
  };

  let days = delta.num_days();
  let hours = delta.num_hours();
  let minutes = delta.num_minutes();

  if minutes == -1 {
    format!("vor {} Minute", minutes.abs())
  } else if minutes == 1 {
    format!("in {minutes} Minute")
  } else if hours == -1 {
    format!("vor {} Stunde", hours.abs())
  } else if hours == 1 {
    format!("in {hours} Stunde")
  } else if days == -1 {
    format!("vor {} Tag", days.abs())
  } else if days == 1 {
    format!("in {days} Tag")
  } else if days < 0 {
    format!("vor {} Tagen", days.abs())
  } else if hours < 0 {
    format!("vor {} Stunden", hours.abs())
  } else if minutes < 0 {
    format!("vor {} Minuten", minutes.abs())
  } else if days > 0 {
    format!("in {days} Tagen")
  } else if hours > 0 {
    format!("in {hours} Stunden")
  } else if minutes > 0 {
    format!("in {minutes} Minuten")
  } else {
    "jetzt".to_string()
  }
}
