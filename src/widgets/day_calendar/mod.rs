use chrono::{NaiveDate, NaiveDateTime};
use relm4::factory::FactoryHashMap;
use gtk::prelude::*;
use relm4::prelude::*;

use crate::calendar::event_uuid::EventUuid;
use crate::calendar::Event;

mod entry;

#[derive(Debug)]
pub struct Widget {
  date: NaiveDate,
  entries: FactoryHashMap<EventUuid, entry::Widget>,
}

#[derive(Debug, Clone)]
pub enum Input {
  Tick(NaiveDateTime),
  Add(Box<Event>),
  Reset,
  SetDay(NaiveDate),
}

#[derive(Debug, Clone)]
pub enum Output {
  RequestEvents(NaiveDate),
}

#[relm4::component(pub)]
impl Component for Widget {
  type Init = NaiveDate;
  type Input = Input;
  type Output = Output;
  type CommandOutput = ();

  view! {
    gtk::ScrolledWindow {
      set_hscrollbar_policy: gtk::PolicyType::Never,
      set_vscrollbar_policy: gtk::PolicyType::Automatic,
      set_child: Some(model.entries.widget()),
      set_vexpand: true,
      set_valign: gtk::Align::Fill,
    },
  }

  fn update(
    &mut self,
    input: Self::Input,
    sender: ComponentSender<Self>,
    _root: &Self::Root,
  ) {
    match input {
      Input::SetDay(date) => {
        self.date = date;
        sender.input(Input::Reset);
      }
      Input::Add(event) => {
        for _ in event.all_matching_between(self.date, self.date) {
          self.entries.insert(event.uid, *event.clone());
        }

        self.entries.widget().invalidate_sort();
      }
      Input::Reset => {
        self.entries.clear();
        sender.output(Output::RequestEvents(self.date)).unwrap();
      }
      Input::Tick(now) => {
        self.entries.broadcast(entry::Input::Tick(now));
      }
    }
  }

  fn init(
    date: Self::Init,
    root: Self::Root,
    _sender: ComponentSender<Self>,
  ) -> ComponentParts<Self> {
    let model = Self {
      date,
      entries: FactoryHashMap::builder()
        .launch(entry::create_parent())
        .detach(),
    };

    let widgets = view_output!();

    ComponentParts { model, widgets }
  }
}
