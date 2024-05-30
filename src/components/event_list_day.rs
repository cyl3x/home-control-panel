use chrono::NaiveDate;
use gtk::{pango, prelude::*};
use relm4::factory::{FactoryHashMap, FactoryVecDeque};
use relm4::prelude::*;

use crate::icalendar::Event;

use super::event_list_day_entry;

#[derive(Debug)]
pub struct Widget {
  date: NaiveDate,
  day_entries: FactoryVecDeque<event_list_day_entry::Widget>,
}

#[derive(Debug, Clone)]
pub enum Input {
  Update,
}

#[derive(Debug, Clone)]
pub enum Output {}

#[relm4::component(pub)]
impl Component for Widget {
  type Init = (NaiveDate);
  type Input = Input;
  type Output = Output;
  type CommandOutput = ();

  view! {
    #[root]
    gtk::Box {
      add_css_class: "calendar-event-list",
      set_orientation: gtk::Orientation::Vertical,
      set_hexpand: true,
      set_halign: gtk::Align::Fill,
      set_valign: gtk::Align::Start,

      gtk::Label {
        add_css_class: "calendar-event-list-day",
        set_hexpand: true,
        set_halign: gtk::Align::Fill,
        set_valign: gtk::Align::Start,
        set_can_focus: false,
        set_ellipsize: pango::EllipsizeMode::End,
        #[watch] set_text: &model.date.format_localized("%A %e %B %Y",chrono::Locale::de_DE).to_string(),
      },

      gtk::Separator {
        set_hexpand: true,
        set_halign: gtk::Align::Fill,
        set_valign: gtk::Align::Start,
      },

      append: model.day_entries.widget(),
    },
  }

  fn update(
    &mut self,
    input: Self::Input,
    _sender: ComponentSender<Self>,
    _root: &Self::Root,
  ) {
    match input {
      Input::Update => {
      }
    }
  }

  fn init(
    (date): Self::Init,
    root: Self::Root,
    _sender: ComponentSender<Self>,
  ) -> ComponentParts<Self> {
    let model = Self {
      date,
      day_entries: FactoryVecDeque::builder().launch_default().detach(),
    };

    let widgets = view_output!();

    ComponentParts { model, widgets }
  }
}
