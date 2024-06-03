use std::collections::BTreeMap;

use chrono::{Datelike, NaiveDate};
use gtk::prelude::*;
use relm4::prelude::*;
use uuid::Uuid;

use crate::calendar::GRID_COLS;
use crate::icalendar::Event;

use super::calendar_event;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridPos {
  pub row: usize,
  pub col: usize,
  pub width: usize,
}

impl GridPos {
  pub const fn range(&self) -> std::ops::Range<usize> {
    self.col..(self.col + self.width)
  }
}

#[derive(Debug)]
pub struct Widget {
  start: NaiveDate,
  end: NaiveDate,
  event_labels: BTreeMap<Uuid, Controller<calendar_event::Widget>>,
  space_manager: SpaceManager,
  day_labels: [gtk::Label; GRID_COLS]
}

#[derive(Debug, Clone)]
pub enum Input {
  Clicked(f64),
}

#[derive(Debug, Clone)]
pub enum Output {
  Clicked(NaiveDate),
}

#[relm4::component(pub)]
impl Component for Widget {
  type Init = [NaiveDate; GRID_COLS];
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
    widgets: &mut Self::Widgets,
    input: Self::Input,
    sender: ComponentSender<Self>,
    _root: &Self::Root,
  ) {
    match input {
      Input::Clicked(x) => {
        // Columns are homogeneous, so we can just divide by the width of the first label
        let col = ((x / self.day_labels[0].width() as f64) as usize).clamp(0, GRID_COLS - 1);

        sender.output(Output::Clicked(self.start + chrono::Duration::days(col as i64))).unwrap();
      },
    }

    self.update_view(widgets, sender);
  }

  fn init(
    dates: Self::Init,
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
      start: dates[0],
      end: dates[GRID_COLS - 1],
      event_labels: BTreeMap::new(),
      space_manager: SpaceManager::new(),
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
  pub fn add_event(&mut self, root: &<Self as Component>::Root, event: &Event) {
    if !event.is_between_dates(self.start, self.end) {
      self.remove_event(&event.uid);

      return;
    }

    let width = event.days_between_dates(self.start, self.end).max(1) as usize;
    let col = (event.start_date().clamp(self.start, self.end) - self.start).num_days() as usize;
    
    let (label_uid, grid_pos) = self.add_or_get(event.clone(), col, width);
    let label = self.event_labels.get(&label_uid).unwrap();

    // Remove the label from the grid before reattaching it
    label.widget().unparent();

    root.attach(label.widget(), grid_pos.col as i32, (grid_pos.row + 1) as i32, grid_pos.width as i32, 1);

    // println!(
    //   "[ROW {}] Attaching '{}' to (r: {}, c: {}) over {} units",
    //   self.idx_start / GRID_COLS,
    //   event.get_summary().unwrap(),
    //   self.next_label_idx,
    //   col,
    //   width,
    // );
  }

  fn add_or_get(&mut self, event: Event, col: usize, width: usize) -> (Uuid, GridPos) {
    let uid = event.uid;

    let grid_pos = self.space_manager.find_and_fill(uid, col, width);

    if let Some(label) = self.event_labels.get(&uid) {
      label.emit(calendar_event::Input::Update(Box::new(event), grid_pos));
    } else {
      let label = calendar_event::Widget::builder().launch((event, grid_pos)).detach();

      self.event_labels.insert(uid, label);
    }

    (uid, grid_pos)
  }

  pub fn remove_event(&mut self, uid: &Uuid) {
    let Some(label) = self.event_labels.remove(uid) else { return };

    let grid_pos = label.model().grid_pos;

    self.space_manager.free(&grid_pos);

    label.widget().unparent();

    drop(label);
  }

  pub fn reset(&mut self) {
    self.space_manager.reset();
    let event_labels = std::mem::take(&mut self.event_labels);
    for (_, label) in event_labels.into_iter() {
      label.widget().unparent();

      drop(label);
    }
  }

  pub fn update_day_labels(&mut self, selected_date: NaiveDate, dates: [NaiveDate; GRID_COLS]) {
    self.start = dates[0];
    self.end = dates[GRID_COLS - 1];

    for (idx, label) in self.day_labels.iter().enumerate() {
      refresh_day_label(label, dates[idx], selected_date);
    }
  }

  pub fn select_day_label(&self, idx: usize, date: NaiveDate, selected_date: NaiveDate) {
    refresh_day_label(&self.day_labels[idx % GRID_COLS], date, selected_date);
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

#[derive(Debug)]
struct SpaceManager(Vec<[Option<Uuid>; GRID_COLS]>);

impl SpaceManager {
  pub fn new() -> Self {
    Self(vec![[None; GRID_COLS]])
  }

  pub fn free(&mut self, grid_pos: &GridPos) {
    self._fill(grid_pos, None);

    if self.0.len() > 1 {
      let last_row = self.0.len() - 1;
      if self.0[last_row].iter().all(|u| u.is_none()) {
        self.0.pop();
      }
    }
  }

  pub fn fill(&mut self, grid_pos: &GridPos, uid: Uuid) {
    if self.0.len() <= grid_pos.row {
      self.0.push([None; GRID_COLS]);
    }

    self._fill(grid_pos, Some(uid));
  }

  pub fn find_free(&self, col: usize, width: usize) -> GridPos {
    let row = self.0
      .iter()
      .enumerate()
      .find_map(|(row_idx, row_tracker)| {
        if row_tracker[col..(col + width)].iter().all(|&v| v.is_none()) {
          Some(row_idx)
        } else {
          None
        }
      })
      .unwrap_or(self.0.len());

    GridPos { row, col, width }
  }

  pub fn find_and_fill(&mut self, uid: Uuid, col: usize, width: usize) -> GridPos {
    let grid_pos = self.find_free(col, width);
    self.fill(&grid_pos, uid);
    grid_pos
  }

  pub fn reset(&mut self) {
    self.0.truncate(1);
    self.0[0].fill(None);
  }

  fn _fill(&mut self, grid_pos: &GridPos, value: Option<Uuid>) {
    self.0[grid_pos.row][grid_pos.range()].fill(value);
  }
}