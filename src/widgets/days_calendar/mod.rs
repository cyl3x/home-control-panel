use chrono::{NaiveDate, NaiveDateTime};
use relm4::factory::FactoryHashMap;
use relm4::prelude::*;

use crate::calendar::Event;

mod day;
mod day_entry;

const DURATION: chrono::Days = chrono::Days::new(13);

#[derive(Debug)]
pub struct Widget {
  start_date: NaiveDate,
  days: FactoryHashMap<NaiveDate, day::Widget>,
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
  RequestEvents(NaiveDate, NaiveDate),
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
      set_child: Some(model.days.widget()),
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
        self.start_date = date;
        sender.input(Input::Reset);
      }
      Input::Add(event) => {
        let end = self.start_date + DURATION;

        for date in event.all_matching_between(self.start_date, end) {
          if self.days.get(&date).is_some() {
            self.days.send(&date, day::Input::Add(event.clone()));
          } else {
            self.days.insert(date, *event.clone());
          }
        }

        self.days.widget().invalidate_sort();
        self.days.broadcast(day::Input::Reorder);
      }
      Input::Reset => {
        self.days.clear();
        sender.output(Output::RequestEvents(self.start_date, self.start_date + DURATION)).unwrap();
      }
      Input::Tick(now) => {
        self.days.broadcast(day::Input::Tick(now));
      }
    }
  }

  fn init(
    start_date: Self::Init,
    root: Self::Root,
    _sender: ComponentSender<Self>,
  ) -> ComponentParts<Self> {
    let model = Self {
      start_date,
      days: FactoryHashMap::builder()
        .launch(day::create_parent())
        .detach(),
    };

    let widgets = view_output!();

    ComponentParts { model, widgets }
  }
}
