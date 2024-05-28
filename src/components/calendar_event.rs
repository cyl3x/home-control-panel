use gtk::{pango, prelude::*};
use relm4::prelude::*;

use crate::icalendar::Event;
use crate::logger::LogExt;

use super::calendar_row::GridPos;

#[derive(Debug, PartialEq)]
pub struct Widget {
  pub grid_pos: GridPos,
  event: Event,
}

#[derive(Debug, Clone)]
pub enum Input {
  Update(Box<Event>, GridPos),
  Refresh,
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
      add_css_class: "calendar-event",
      set_hexpand: true,
      set_halign: gtk::Align::Fill,
      set_valign: gtk::Align::Start,
      set_can_focus: false,
      set_ellipsize: pango::EllipsizeMode::End,
      #[watch] set_tooltip: &model.event.summary,
      #[watch] set_text: &model.event.summary,
    }
  }

  fn update(
    &mut self,
    input: Self::Input,
    sender: ComponentSender<Self>,
    root: &Self::Root,
  ) {
    match input {
      Input::Update(event, grid_pos) => {
        self.event = *event;
        self.grid_pos = grid_pos;
        sender.input(Input::Refresh);
      }
      Input::Refresh => {
        let bg_color = self.event.color.as_deref().unwrap_or("#ffffff");
        let fg_color = fg_from_bg_w3c(bg_color).log_warn("Event invalid background color");
        
        if let Ok(fg_color) = fg_color {
          root.inline_css(&format!("background-color: {}; color: {};", bg_color, fg_color));
        } else {
          root.inline_css("");
        }
      }
    }
  }

  fn init(
    (event, grid_pos): Self::Init,
    root: Self::Root,
    sender: ComponentSender<Self>,
  ) -> ComponentParts<Self> {
    let model = Self { event, grid_pos };

    let widgets = view_output!();

    sender.input(Input::Refresh);

    ComponentParts { model, widgets }
  }
}

fn fg_from_bg_w3c<'a>(bg_color: &str) -> Option<&'a str> {
  let color = if bg_color.starts_with('#') { &bg_color[1..bg_color.len()] } else { bg_color };

  let mut rgb = [
    i32::from_str_radix(&color[0..2], 16).ok()? as f32 / 255.0,
    i32::from_str_radix(&color[2..4], 16).ok()? as f32 / 255.0,
    i32::from_str_radix(&color[4..6], 16).ok()? as f32 / 255.0,
  ];

   rgb = rgb.map(|c| {
    if c <= 0.04045 {
      c / 12.92
    } else {
      ((c + 0.055) / 1.055).powf(2.4)
    }
  });

  if rgb[0].mul_add(0.2126, rgb[1].mul_add(0.7152, rgb[2] * 0.0722)) > 0.179 {
    Some("#000000")
  } else {
    Some("#ffffff")
  }
}