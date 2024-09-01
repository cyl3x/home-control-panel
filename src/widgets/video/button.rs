use gtk::prelude::*;
use relm4::prelude::*;

#[derive(Debug)]
pub struct Widget {
  name: String,
}

#[derive(Debug, Clone)]
pub enum Input {}

#[derive(Debug, Clone)]
pub enum Output {
  Clicked(usize),
}

#[relm4::factory(pub)]
impl FactoryComponent for Widget {
  type Init = String;
  type Input = Input;
  type Output = Output;
  type ParentWidget = gtk::Box;
  type CommandOutput = ();

  view! {
    gtk::Button {
      set_label: &self.name,
      set_hexpand: true,
      set_size_request: (-1, 50),
      connect_clicked[sender, index] => move |_| {
        sender.output(Output::Clicked(index.current_index())).unwrap();
      }
    },
  }

  fn init_model(name: Self::Init, _index: &Self::Index, _sender: FactorySender<Self>) -> Self {
    Self { name }
  }
}

pub fn create_parent() -> gtk::Box {
  gtk::Box::builder()
    .orientation(gtk::Orientation::Horizontal)
    .spacing(8)
    .margin_top(4)
    .margin_end(4)
    .margin_bottom(4)
    .margin_start(4)
    .build()
}
