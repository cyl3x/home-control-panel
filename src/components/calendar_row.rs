use chrono::{Datelike, NaiveDate};
use gtk::prelude::*;
use relm4::prelude::*;
use icalendar::{Component as _, Event};

use crate::calendar::{GRID_COLS, EVENT_COLOR};

use super::calendar_event;

#[derive(Debug)]
pub struct Widget {
  idx_start: usize,
  idx_end: usize,
  next_label_idx: usize,
  event_labels: Vec<Controller<calendar_event::Widget>>,
  used_space: Vec<[bool; GRID_COLS]>,
  day_labels: [gtk::Label; GRID_COLS]
}

#[derive(Debug, Clone)]
pub enum Input {
  UpdateDates(NaiveDate, [NaiveDate; GRID_COLS]),
  Select(usize, NaiveDate, NaiveDate),
  Clicked(f64),
  Reset,
  Finish,
  Add((usize, usize, Event))
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Output {
  Clicked(usize),
}

#[relm4::component(pub)]
impl Component for Widget {
  type Init = usize;
  type Input = Input;
  type Output = Output;
  type CommandOutput = ();

  view! {
    #[root]
    gtk::Grid {
      set_halign: gtk::Align::Fill,
      set_valign: gtk::Align::Fill,
      set_hexpand: true,
      set_vexpand: true,
      set_column_homogeneous: true,
      set_row_spacing: 4,
    }
  }

  fn update_with_view(
    &mut self,
    _widgets: &mut Self::Widgets,
    input: Self::Input,
    sender: ComponentSender<Self>,
    root: &Self::Root,
  ) {
    match input {
      Input::Clicked(x) => {
        // Columns are homogeneous, so we can just divide by the width of the first label
        let col = ((x / self.day_labels[0].width() as f64) as usize).clamp(0, GRID_COLS - 1);

        sender.output(Output::Clicked(self.idx_start + col)).unwrap();
      },
      Input::UpdateDates(selected_date, dates) => {
        for (idx, label) in self.day_labels.iter().enumerate() {
          refresh_day_label(label, dates[idx], selected_date);
        }
      },
      Input::Select(idx, date, selected_date) => {
        refresh_day_label(&self.day_labels[idx % GRID_COLS], date, selected_date);
      },
      Input::Reset => {
        self.next_label_idx = 0;
        self.used_space.truncate(1);
        self.used_space[0].fill(false);
        for label in &self.event_labels {
          root.remove(label.widget());
        }
      },
      Input::Add((event_start_idx, event_end_idx, event)) => {
        if event_start_idx > self.idx_end || event_end_idx < self.idx_start {
          return;
        }

        let start_clamped = event_start_idx.clamp(self.idx_start, self.idx_end + 1);
        let end_clamped = event_end_idx.clamp(self.idx_start, self.idx_end + 1);
        let width = end_clamped - start_clamped;
        let col = start_clamped - self.idx_start;

        if width == 0 {
          return; // event ends before the start of the row
        }

        let label_idx = self.get_next_label(&event);
        let label = self.event_labels.get(label_idx).unwrap();
        
        let mut row = 0;
        for (row_idx, row_tracker) in &mut self.used_space.iter().enumerate() {
          row = row_idx;

          if row_tracker[col..(col + width)].iter().all(|&v| !v) {
            break;
          }
        }

        self.used_space[row][col..(col + width)].fill(true);

        root.attach(label.widget(), col as i32, (row + 1) as i32, width as i32, 1);

        // println!(
        //   "[ROW {}] Attaching '{}' to (r: {}, c: {}) over {} units",
        //   self.idx_start / GRID_COLS,
        //   event.get_summary().unwrap(),
        //   self.next_label_idx,
        //   col,
        //   width,
        // );
      },
      Input::Finish => {
        for idx in self.next_label_idx..self.event_labels.len() {
          self.event_labels[idx].widget().unparent();
        }

        self.event_labels.truncate(self.next_label_idx.min(3));
      }
    }
  }

  fn init(
    idx_start: Self::Init,
    root: Self::Root,
    sender: ComponentSender<Self>,
  ) -> ComponentParts<Self> {
    let day_labels = [(); GRID_COLS].map(|_| {
      gtk::Label::builder().css_classes(["calendar-day"]).hexpand(true).build()
    });
  
    for (col_idx, label) in day_labels.iter().enumerate() {
      root.attach(label, col_idx as i32, 0, 1, 1);
    }

    let model = Self {
      day_labels,
      idx_start,
      idx_end: idx_start + GRID_COLS - 1,
      event_labels: vec![],
      used_space: vec![[false; GRID_COLS]; 1],
      next_label_idx: 0,
    };

    let widgets = view_output!();

    let controller = gtk::GestureClick::default();
    controller.connect_pressed(gtk::glib::clone!(@strong sender => move |controller, _, x, _| {
      if controller.current_button() == gtk::gdk::BUTTON_PRIMARY {
        sender.input(Input::Clicked(x));
      }
    }));

    root.add_controller(controller);

    ComponentParts { model, widgets }
  }
}

impl Widget {
  fn get_next_label(&mut self, event: &Event) -> usize {
    let summary = event.get_summary().unwrap_or("<no title>").to_string();
    let color = event.property_value(EVENT_COLOR).map(|v| v.to_string());

    if let Some(label) = self.event_labels.get(self.next_label_idx) {
      label.emit(calendar_event::Input::Update(summary, color));
    } else {
      let label = calendar_event::Widget::builder().launch((summary, color)).detach();

      self.event_labels.insert(self.next_label_idx, label);
    }

    self.next_label_idx += 1;
    
    self.next_label_idx - 1
  }
}

fn refresh_day_label(widget: &gtk::Label, date: NaiveDate, selected_date: NaiveDate) {
  widget.set_text(&date.day().to_string());

  if date.month() != selected_date.month() {
    widget.add_css_class("day-other-month");
  } else {
    widget.remove_css_class("day-other-month");
  }

  // if self.date ==  {
  //   widget.add_css_class("day-today");
  // } else {
  //   widget.remove_css_class("day-today");
  // }

  if date == selected_date {
    widget.add_css_class("day-selected");
  } else {
    widget.remove_css_class("day-selected");
  }
}