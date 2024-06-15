use gtk::prelude::*;
use relm4::prelude::*;
use uuid::Uuid;

use crate::calendar::Calendar;

#[derive(Debug)]
pub struct Widget {
  calendar: Calendar,
  is_active: bool,
}

#[derive(Debug, Clone)]
pub enum Input {
  Clicked,
}

#[derive(Debug, Clone)]
pub enum Output {
  Clicked(Uuid, bool)
}

#[relm4::factory(pub)]
impl FactoryComponent for Widget {
  type Init = Calendar;
  type Input = Input;
  type Output = Output;
  type CommandOutput = ();
  type ParentWidget = gtk::Box;
  type Index = Uuid;

  view! {
    gtk::Label {
      add_css_class: "calendar-selection-label",
      #[watch] inline_css: &format!("border-top: 4px solid {};", self.calendar.color()),
      #[watch] inline_css: if self.is_active { "filter: none" } else { "filter: brightness(50%)" },
      #[watch] set_text: &self.calendar.name,
      set_hexpand: true,
      set_halign: gtk::Align::Fill,
      set_can_focus: false,

      add_controller = gtk::GestureClick {
        connect_released[sender] => move |controller, _, _, _| {
          if controller.current_button() == gtk::gdk::BUTTON_PRIMARY {
            sender.input(Input::Clicked);
          }
        },
      },
    }
  }

  fn update(&mut self, input: Self::Input, sender: FactorySender<Self>) {
    match input {
      Input::Clicked => {
        self.is_active = !self.is_active;
        sender.output(Output::Clicked(self.calendar.uid, self.is_active)).unwrap();
      }
    }
  }

  fn init_model(calendar: Self::Init, _index: &Self::Index, _sender: FactorySender<Self>) -> Self {
    Self { calendar, is_active: true }
  }
}

pub fn create_parent() -> gtk::Box {
  gtk::Box::builder()
    .orientation(gtk::Orientation::Horizontal)
    .hexpand(true)
    .margin_bottom(4)
    .build()
}
