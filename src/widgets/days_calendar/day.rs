use chrono::{NaiveDate, NaiveDateTime};
use gtk::{pango, prelude::*, ListBoxRow};
use relm4::factory::FactoryHashMap;
use relm4::prelude::*;

use crate::calendar::event_uuid::EventUuid;
use crate::calendar::Event;

use super::day_entry;

#[derive(Debug)]
pub struct Widget {
  date: NaiveDate,
  day_entries: FactoryHashMap<EventUuid, day_entry::Widget>,
}

#[derive(Debug, Clone)]
pub enum Input {
  Add(Box<Event>),
  Tick(NaiveDateTime),
  Reorder,
}

#[derive(Debug, Clone)]
pub enum Output {}

#[relm4::factory(pub)]
impl FactoryComponent for Widget {
  type Init = Event;
  type Input = Input;
  type Output = Output;
  type CommandOutput = ();
  type ParentWidget = gtk::ListBox;
  type Index = NaiveDate;

  view! {
    gtk::Box {
      add_css_class: "days-calendar-day",
      set_orientation: gtk::Orientation::Vertical,
      set_halign: gtk::Align::Fill,
      set_hexpand: true,
      set_spacing: 4,
      #[watch] set_widget_name: &self.date.to_string(),

      gtk::Label {
        add_css_class: "days-calendar-day-label",
        set_hexpand: true,
        set_halign: gtk::Align::Start,
        set_can_focus: false,
        set_ellipsize: pango::EllipsizeMode::End,
        #[watch] set_text: &self.date.format_localized("%A %e %B %Y", chrono::Locale::de_DE).to_string(),
      },

      gtk::Separator {
        set_hexpand: true,
      },

      append: self.day_entries.widget(),
    },
  }

  fn init_model(event: Self::Init, index: &Self::Index, _sender: FactorySender<Self>) -> Self {
    let list_box = gtk::ListBox::new();
    list_box.set_selection_mode(gtk::SelectionMode::None);
    list_box.set_sort_func(sort_func);

    let mut day_entries = FactoryHashMap::builder().launch(list_box).detach();
    day_entries.insert(event.uid, (*index, event));

    Self {
      date: *index,
      day_entries,
    }
  }

  fn update(&mut self, input: Self::Input, _sender: FactorySender<Self>) {
    match input {
      Input::Reorder => self.day_entries.widget().invalidate_sort(),
      Input::Tick(now) => self.day_entries.broadcast(day_entry::Input::Tick(now)),
      Input::Add(event) => {
        self.day_entries.insert(event.uid, (self.date, *event));
        self.day_entries.widget().invalidate_sort();
      }
    }
  }
}

pub fn create_parent() -> gtk::ListBox {
  let list = gtk::ListBox::builder()
    .halign(gtk::Align::Fill)
    .valign(gtk::Align::Start)
    .hexpand(true)
    .selection_mode(gtk::SelectionMode::None)
    .css_classes(["days-calendar"])
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
