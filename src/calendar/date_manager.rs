use chrono::{Datelike, NaiveDate};

pub const GRID_ROWS: usize = 6;
pub const GRID_COLS: usize = 7;
pub const GRID_LENGTH: usize = GRID_ROWS * GRID_COLS;

#[derive(Debug, PartialEq, Eq)]
pub struct DateManager {
  date_pointer: usize,
  month_grid: [NaiveDate; GRID_LENGTH],
}

impl DateManager {
  pub fn new(date: NaiveDate) -> Self {
    let mut calendar = Self {
      date_pointer: 0,
      month_grid: [NaiveDate::default(); GRID_LENGTH],
    };

    calendar.date_pointer = calendar.regenerate_month_grid(date);

    calendar
  }

  pub const fn current(&self) -> NaiveDate {
    self.month_grid[self.date_pointer]
  }

  pub const fn current_idx(&self) -> usize {
    self.date_pointer
  }

  pub const fn first(&self) -> NaiveDate {
    self.month_grid[0]
  }

  pub const fn last(&self) -> NaiveDate {
    self.month_grid[GRID_LENGTH - 1]
  }

  pub const fn date(&self, idx: usize) -> NaiveDate {
    self.month_grid[idx]
  }

  pub const fn month_grid(&self) -> &[NaiveDate; GRID_LENGTH] {
    &self.month_grid
  }

  pub fn row(&self, row_idx: usize) -> [NaiveDate; GRID_COLS] {
    let idx = row_idx * GRID_COLS;
    self.month_grid[idx..(idx + GRID_COLS)].try_into().expect("Row is always correct length")
  }

  /// Set the date of the calendar
  /// Returns the index of the date in the month grid, if the month has changed
  pub fn set_date(&mut self, date: NaiveDate) -> Option<usize> {
    let old_date = self.month_grid[self.date_pointer];
    if old_date.month0() == date.month0() {
      self.date_pointer = self.date_pointer + date.day0() as usize - old_date.day0() as usize;

      None
    } else {
      self.date_pointer = self.regenerate_month_grid(date);

      Some(self.date_pointer)
    }
  }

  pub fn next_month(&mut self) -> NaiveDate
  {
    let new_date = self
      .month_grid[self.date_pointer]
      .checked_add_months(chrono::Months::new(1))
      .unwrap_or_else(|| self.month_grid[self.date_pointer]);

    self.set_date(new_date);

    new_date
  }

  pub fn prev_month(&mut self) -> NaiveDate
  {
    let new_date = self
      .month_grid[self.date_pointer]
      .checked_sub_months(chrono::Months::new(1))
      .unwrap_or_else(|| self.month_grid[self.date_pointer]);

    self.set_date(new_date);

    new_date
  }

  pub fn regenerate_month_grid(&mut self, date: NaiveDate) -> usize {
    let first = first_grid_date(date);
    let last = first
      .checked_add_days(chrono::Days::new(GRID_LENGTH as u64 - 1))
      .unwrap_or(first);

    let mut index: Option<usize> = None;
    first
      .iter_days()
      .take_while(|d| d <= &last)
      .enumerate()
      .for_each(|(i, d)| {
        if d == date {
          index = Some(i);
        }

        self.month_grid[i] = d;
      });

    index.expect("Date is always in the grid")
  }
}

pub fn first_grid_date(date: NaiveDate) -> NaiveDate {
  let mut first = date.with_day(1).unwrap();

  while first.weekday() != chrono::Weekday::Mon {
    first = first.pred_opt().unwrap_or(first);
  }

  first
}