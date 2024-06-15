use std::collections::BTreeMap;

use chrono::{Datelike, NaiveDate, NaiveDateTime};
use gtk::prelude::*;
use relm4::prelude::*;
use uuid::Uuid;

use crate::calendar::Event;

mod event;

const DURATION: chrono::Days = chrono::Days::new(6);

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
  selected: NaiveDate,
  event_labels: BTreeMap<Uuid, Controller<event::Widget>>,
  space_manager: SpaceManager,
}

#[derive(Debug, Clone)]
pub enum Input {
  Tick(NaiveDateTime),
  SetDay(NaiveDate),
  Add(Event),
  Reset,
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
    #[root]
    gtk::Grid {
      add_css_class: "week-calendar",
      set_hexpand: true,
      set_vexpand: true,
      set_column_homogeneous: true,
      set_row_spacing: 4,

      attach[0, 0, 1, 1] = &gtk::Label { add_css_class: "week-calendar-weekday", set_text: "Mo", },
      attach[1, 0, 1, 1] = &gtk::Label { add_css_class: "week-calendar-weekday", set_text: "Di", },
      attach[2, 0, 1, 1] = &gtk::Label { add_css_class: "week-calendar-weekday", set_text: "Mi", },
      attach[3, 0, 1, 1] = &gtk::Label { add_css_class: "week-calendar-weekday", set_text: "Do", },
      attach[4, 0, 1, 1] = &gtk::Label { add_css_class: "week-calendar-weekday", set_text: "Fr", },
      attach[5, 0, 1, 1] = &gtk::Label { add_css_class: "week-calendar-weekday", set_text: "Sa", },
      attach[6, 0, 1, 1] = &gtk::Label { add_css_class: "week-calendar-weekday", set_text: "So", },
    }
  }

  fn update_with_view(
    &mut self,
    widgets: &mut Self::Widgets,
    input: Self::Input,
    sender: ComponentSender<Self>,
    root: &Self::Root,
  ) {
    match input {
      Input::SetDay(date) => {
        self.selected = date;
        sender.input(Input::Reset);
      }
      Input::Add(event) => {
        let start = start_week_date(self.selected);
        let end = start + DURATION;

        let width = event.days_between_dates(start, end).max(1) as usize;
        let col = (event.start_date().clamp(start, end) - start).num_days() as usize;

        let (label_uid, grid_pos) = self.add_or_get(event, col, width);
        let label = self.event_labels.get(&label_uid).unwrap();

        root.attach(label.widget(), grid_pos.col as i32, (grid_pos.row + 1) as i32, grid_pos.width as i32, 1);
      }
      Input::Reset => {
        self.space_manager.reset();

        let event_labels = std::mem::take(&mut self.event_labels);

        for (_, label) in event_labels {
          label.widget().unparent();

          drop(label);
        }

        let start = start_week_date(self.selected);
        sender.output(Output::RequestEvents(start, start + DURATION)).unwrap();
      }
      Input::Tick(_now) => {}
    }

    self.update_view(widgets, sender);
  }

  fn init(
    selected: Self::Init,
    root: Self::Root,
    _sender: ComponentSender<Self>,
  ) -> ComponentParts<Self> {
    let model = Self {
      selected,
      event_labels: BTreeMap::new(),
      space_manager: SpaceManager::new(),
    };

    let widgets = view_output!();

    ComponentParts { model, widgets }
  }
}

impl Widget {
  fn add_or_get(&mut self, event: Event, col: usize, width: usize) -> (Uuid, GridPos) {
    let uid = event.uid;

    let grid_pos = self.space_manager.find_and_fill(uid, col, width);

    if let Some(label) = self.event_labels.get(&uid) {
      label.emit(event::Input::Update(event, grid_pos));
    } else {
      let label = event::Widget::builder().launch((event, grid_pos)).detach();

      self.event_labels.insert(uid, label);
    }

    (uid, grid_pos)
  }
}

fn start_week_date(date: NaiveDate) -> NaiveDate {
  let mut date = date;
  while date.weekday() != chrono::Weekday::Mon {
    date = date.pred_opt().unwrap_or(date);
  }

  date
}

#[derive(Debug)]
struct SpaceManager(Vec<[Option<Uuid>; 7]>);

impl SpaceManager {
  pub fn new() -> Self {
    Self(vec![[None; 7]])
  }

  pub fn free(&mut self, grid_pos: &GridPos) {
    self._fill(grid_pos, None);

    if self.0.len() > 1 {
      let last_row = self.0.len() - 1;
      if self.0[last_row].iter().all(std::option::Option::is_none) {
        self.0.pop();
      }
    }
  }

  pub fn fill(&mut self, grid_pos: &GridPos, uid: Uuid) {
    if self.0.len() <= grid_pos.row {
      self.0.push([None; 7]);
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
