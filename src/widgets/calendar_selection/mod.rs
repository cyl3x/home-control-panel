use relm4::factory::FactoryHashMap;
use relm4::prelude::*;
use uuid::Uuid;

use crate::calendar::Calendar;

mod calendar;

#[derive(Debug)]
pub struct Widget {
  calendars: FactoryHashMap<Uuid, calendar::Widget>,
}

#[derive(Debug, Clone)]
pub enum Input {
  Add(Box<Calendar>),
  Reset,
}

#[derive(Debug, Clone)]
pub enum Output {
  RequestCalendars,
  Clicked(Uuid, bool),
}

#[relm4::component(pub)]
impl Component for Widget {
  type Init = ();
  type Input = Input;
  type Output = Output;
  type CommandOutput = ();

  view! {
    gtk::ScrolledWindow {
      set_hscrollbar_policy: gtk::PolicyType::Automatic,
      set_vscrollbar_policy: gtk::PolicyType::Never,
      set_child: Some(model.calendars.widget()),
    },
  }

  fn update(&mut self, input: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
    match input {
      Input::Add(calendar) => {
        self.calendars.insert(calendar.uid, *calendar);
      }
      Input::Reset => {
        self.calendars.clear();
        sender.output(Output::RequestCalendars).unwrap();
      }
    }
  }

  fn init(_: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
    let model = Self {
      calendars: FactoryHashMap::builder().launch(calendar::create_parent()).forward(sender.output_sender(), |output| match output {
        calendar::Output::Clicked(uid, is_active) => Output::Clicked(uid, is_active),
      }),
    };

    let widgets = view_output!();

    ComponentParts { model, widgets }
  }
}
