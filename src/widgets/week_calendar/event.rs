use gtk::{pango, prelude::*};
use relm4::prelude::*;

use crate::icalendar::Event;

use super::GridPos;

#[derive(Debug, PartialEq)]
pub struct Widget {
  pub grid_pos: GridPos,
  event: Event,
}

#[derive(Debug, Clone)]
pub enum Input {
  Update(Event, GridPos),
}

#[derive(Debug, Clone)]
pub enum Output {}

#[relm4::component(pub)]
impl Component for Widget {
  type Init = (Event, GridPos);
  type Input = Input;
  type Output = Output;
  type CommandOutput = ();

  view! {
    #[root]
    gtk::Label {
      inline_css: "border-radius: 4px; padding: 4px;",
      #[watch] inline_css: &format!("background-color: {}; color: {};", model.event.color(), model.event.fg_color()),
      set_hexpand: true,
      set_halign: gtk::Align::Fill,
      set_valign: gtk::Align::Start,
      set_can_focus: false,
      set_ellipsize: pango::EllipsizeMode::End,
      #[watch] set_tooltip: &model.event.tooltip(),
      #[watch] set_text: &model.event.summary,
    }
  }

  fn update(
    &mut self,
    input: Self::Input,
    _sender: ComponentSender<Self>,
    _root: &Self::Root,
  ) {
    match input {
      Input::Update(event, grid_pos) => {
        self.event = event;
        self.grid_pos = grid_pos;
      }
    }
  }

  fn init(
    (event, grid_pos): Self::Init,
    root: Self::Root,
    _sender: ComponentSender<Self>,
  ) -> ComponentParts<Self> {
    let model = Self { grid_pos, event };

    let widgets = view_output!();

    ComponentParts { model, widgets }
  }
}
