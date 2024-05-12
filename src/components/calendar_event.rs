use gtk::{pango, prelude::*};
use relm4::prelude::*;

#[derive(Debug, PartialEq, Eq)]
pub struct Widget {
  summary: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Input {
  Update(String, Option<String>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Output {}

#[relm4::component(pub)]
impl Component for Widget {
  type Init = (String, Option<String>);
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
      #[watch] set_tooltip: &model.summary,
      #[watch] set_text: &model.summary,
    }
  }

  fn update(
    &mut self,
    input: Self::Input,
    _sender: ComponentSender<Self>,
    root: &Self::Root,
  ) {
    match input {
      Input::Update(summary, color) => {
        self.summary = summary;

        let bg_color = color.unwrap_or_else(|| "#ffffff".to_string());
        let fg_color = fg_from_bg_w3c(&bg_color);

        root.inline_css(&format!("background-color: {}; color: {};", bg_color, fg_color));
      }
    }
  }

  fn init(
    (summary, color): Self::Init,
    root: Self::Root,
    sender: ComponentSender<Self>,
  ) -> ComponentParts<Self> {
    let model = Self { summary: String::new() };

    let widgets = view_output!();
    
    sender.input(Input::Update(summary, color));

    ComponentParts { model, widgets }
  }
}

fn fg_from_bg_w3c<'a>(bg_color: &str) -> &'a str {
  let color = if bg_color.starts_with('#') { &bg_color[1..bg_color.len()] } else { bg_color };

  let mut rgb = [
    i32::from_str_radix(&color[0..2], 16).unwrap() as f32 / 255.0,
    i32::from_str_radix(&color[2..4], 16).unwrap() as f32 / 255.0,
    i32::from_str_radix(&color[4..6], 16).unwrap() as f32 / 255.0,
  ];

   rgb = rgb.map(|c| {
    if c <= 0.04045 {
      c / 12.92
    } else {
      ((c + 0.055) / 1.055).powf(2.4)
    }
  });

  if rgb[0].mul_add(0.2126, rgb[1].mul_add(0.7152, rgb[2] * 0.0722)) > 0.179 {
    "#000000"
  } else {
    "#ffffff"
  }
}