use std::ops::RangeInclusive;

use chrono::{DateTime, Datelike as _, Days, Months, NaiveDate, Utc, Weekday};

use crate::icalendar::EARLIEST_NAIVE_TIME;

pub const GRID_ROWS: usize = 6;
pub const GRID_COLS: usize = 7;
pub const GRID_LENGTH: usize = GRID_ROWS * GRID_COLS;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GridService {
  start: NaiveDate,
  end: NaiveDate,
  current: NaiveDate,
}

impl GridService {
  pub fn new(current: NaiveDate) -> Self {
    let start = Self::start_grid_date(current);
    let end = Self::end_grid_date(start);
    Self {
      start,
      end,
      current,
    }
  }

  pub const fn start(&self) -> NaiveDate {
    self.start
  }

  pub const fn end(&self) -> NaiveDate {
    self.end
  }

  pub const fn start_time(&self) -> DateTime<Utc> {
    self.start.and_time(EARLIEST_NAIVE_TIME).and_utc()
  }

  pub fn end_time(&self) -> DateTime<Utc> {
    (self.end + Days::new(1)).and_time(EARLIEST_NAIVE_TIME).and_utc()
  }

  pub const fn start_end(&self) -> (NaiveDate, NaiveDate) {
    (self.start, self.end)
  }

  pub const fn current(&self) -> NaiveDate {
    self.current
  }

  pub fn current_idx(&self) -> usize {
    self.date_to_idx(self.current)
  }

  pub fn current_row_idx(&self) -> usize {
    self.row_idx(self.current)
  }

  pub fn current_col_idx(&self) -> usize {
    self.col_idx(self.current)
  }

  pub fn row_idx(&self, date: NaiveDate) -> usize {
    self.date_to_idx(date) / GRID_COLS
  }

  pub fn col_idx(&self, date: NaiveDate) -> usize {
    self.date_to_idx(date) % GRID_COLS
  }

  pub fn row_date(&self, row_idx: usize) -> NaiveDate {
    self.idx_to_date(row_idx * GRID_COLS)
  }

  pub fn row(&self, row_idx: usize) -> [NaiveDate; GRID_COLS] {
    self.row_date(row_idx)
      .iter_days()
      .take(GRID_COLS)
      .collect::<Vec<_>>()
      .try_into()
      .expect("Row is always correct length")
  }

  pub fn intersecting_rows(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> RangeInclusive<usize> {
    let start_idx = self.date_time_to_idx(start);
    let end_idx = self.date_time_to_idx(end);

    let start_row = start_idx / GRID_COLS;
    let end_row = end_idx / GRID_COLS;

    start_row..=end_row.clamp(0, GRID_ROWS - 1)
  }

  /// Set the date of the calendar
  /// Returns the index of the date in the month grid, if the month has changed
  pub fn set_date(&mut self, date: NaiveDate) -> Option<usize> {
    if self.current.month() == date.month() {
      self.current = date;

      None
    } else {
      self.current = date;
      self.start = Self::start_grid_date(date);
      self.end = Self::end_grid_date(self.start);

      Some(self.current_idx())
    }
  }

  pub fn next_month(&mut self) -> NaiveDate
  {
    let new_date = self.current + Months::new(1);

    self.set_date(new_date);

    new_date
  }

  pub fn prev_month(&mut self) -> NaiveDate
  {
    let new_date = self.current - Months::new(1);

    self.set_date(new_date);

    new_date
  }

  fn idx_to_date(&self, idx: usize) -> NaiveDate {
    (self.start + Days::new(idx as u64)).clamp(self.start, self.end)
  }

  fn date_to_idx(&self, date: NaiveDate) -> usize {
    ((date - self.start).num_days() as usize).clamp(0, GRID_LENGTH - 1)
  }

  fn date_time_to_idx(&self, date: DateTime<Utc>) -> usize {
    ((date - self.start_time()).num_days() as usize).clamp(0, GRID_LENGTH - 1)
  }

  fn start_grid_date(date: NaiveDate) -> NaiveDate {
    let mut first = date.with_day(1).unwrap();
  
    while first.weekday() != Weekday::Mon {
      first = first.pred_opt().unwrap_or(first);
    }
  
    first
  }
  
  fn end_grid_date(start: NaiveDate) -> NaiveDate {
    start + Days::new(GRID_LENGTH as u64 - 1)
  }
}