use chrono::{Datelike, Days, NaiveDate, NaiveDateTime};
use gtk::prelude::*;
use relm4::prelude::*;

use crate::calendar::Event;

mod day;

const DURATION: chrono::Days = chrono::Days::new(41);

#[derive(Debug)]
pub struct Widget {
  selected: NaiveDate,
  now_date: NaiveDate,
  days: [Controller<day::Widget>; 42],
}

#[derive(Debug, Clone)]
pub enum Input {
  Tick(NaiveDateTime),
  NextMonth,
  PreviousMonth,
  Select(NaiveDate),
  Add(Event),
  Reset,
}

#[derive(Debug, Clone)]
pub enum Output {
  Selected(NaiveDate),
  RequestEvents(NaiveDate, NaiveDate),
}

#[relm4::component(pub)]
impl Component for Widget {
  type Init = NaiveDate;
  type Input = Input;
  type Output = Output;
  type CommandOutput = ();

  view! {
    gtk::Box {
      add_css_class: "month-calendar",
      set_orientation: gtk::Orientation::Vertical,
      set_spacing: 16,

      gtk::Box {
        #[name(prev_button)]
        gtk::Button {
          set_icon_name: "pan-start-symbolic",
          set_size_request: (64, 52),
          set_halign: gtk::Align::Start,

          connect_clicked => Input::PreviousMonth,
        },

        #[name(month_label)]
        gtk::Label {
          add_css_class: "month-calendar-label",
          set_hexpand: true,
          set_halign: gtk::Align::Fill,
          #[watch] set_text: &model.selected.format_localized("%B %Y", chrono::Locale::de_DE).to_string(),
        },

        #[name(next_button)]
        gtk::Button {
          set_icon_name: "pan-end-symbolic",
          set_size_request: (64, 52),
          set_halign: gtk::Align::Start,

          connect_clicked => Input::NextMonth,
        },
      },

      #[name(calendar_grid)]
      gtk::Grid {
        add_css_class: "calendar-grid",

        set_valign: gtk::Align::Start,
        set_hexpand: true,
        set_vexpand: true,
        set_column_homogeneous: true,

        attach[0, 0, 1, 1] = &gtk::Label { add_css_class: "month-calendar-weekday", set_text: "Mo", },
        attach[1, 0, 1, 1] = &gtk::Label { add_css_class: "month-calendar-weekday", set_text: "Di", },
        attach[2, 0, 1, 1] = &gtk::Label { add_css_class: "month-calendar-weekday", set_text: "Mi", },
        attach[3, 0, 1, 1] = &gtk::Label { add_css_class: "month-calendar-weekday", set_text: "Do", },
        attach[4, 0, 1, 1] = &gtk::Label { add_css_class: "month-calendar-weekday", set_text: "Fr", },
        attach[5, 0, 1, 1] = &gtk::Label { add_css_class: "month-calendar-weekday", set_text: "Sa", },
        attach[6, 0, 1, 1] = &gtk::Label { add_css_class: "month-calendar-weekday", set_text: "So", },
      },
    },
  }

  fn update_with_view(
    &mut self,
    widgets: &mut Self::Widgets,
    input: Self::Input,
    sender: ComponentSender<Self>,
    _root: &Self::Root,
  ) {
    match input {
      Input::Add(event) => {
        let start = start_grid_date(self.selected);

        for date in event.all_matching_between(start, start + DURATION) {
          self.days[date_to_idx(start, date)].emit(day::Input::Add(event.clone()));
        }
      }
      Input::Reset => {
        let start = start_grid_date(self.selected);

        for day in &self.days {
          day.emit(day::Input::Reset);
        }

        sender.output(Output::RequestEvents(start, start + DURATION)).unwrap();
      }
      Input::NextMonth => {
        let new = self.selected + chrono::Months::new(1);
        sender.input(Input::Select(new));
      }
      Input::PreviousMonth => {
        let new = self.selected - chrono::Months::new(1);
        sender.input(Input::Select(new));
      }
      Input::Select(date) => {
        let start = start_grid_date(date);

        if self.selected.month() != date.month() {
          for (idx, day) in self.days.iter_mut().enumerate() {
            let day_date = start + Days::new(idx as u64);
            day.emit(day::Input::SetDay(day_date, day_date.month() == date.month()));
            day.emit(day::Input::Reset);
          }

          sender.output(Output::RequestEvents(start, start + DURATION)).unwrap();
        }

        self.days[date_to_idx(start, self.selected)].emit(day::Input::Deselect);
        self.days[date_to_idx(start, date)].emit(day::Input::Select);
        self.selected = date;

        sender.output(Output::Selected(date)).unwrap();
      }
      Input::Tick(now) => {
        for day in &self.days {
          day.emit(day::Input::Tick(now));
        }

        if now.date() != self.now_date {
          self.now_date = now.date();
          sender.input(Input::Select(self.now_date));
        }
      }
    }

    self.update_view(widgets, sender);
  }

  fn init(
    selected: Self::Init,
    root: Self::Root,
    sender: ComponentSender<Self>,
  ) -> ComponentParts<Self> {
    let model = Self {
      now_date: chrono::Utc::now().date_naive(),
      selected: NaiveDate::default(),
      days: std::array::from_fn(|_| day::Widget::builder()
        .launch(())
        .forward(sender.input_sender(), |output| match output {
          day::Output::Selected(day) => Input::Select(day),
        })),
    };

    let widgets = view_output!();

    for (idx, day) in model.days.iter().enumerate() {
      widgets.calendar_grid.attach(day.widget(), (idx % 7) as i32, (idx / 7) as i32 + 1, 1, 1);
    }

    sender.input(Input::Select(selected));

    ComponentParts { model, widgets }
  }
}

fn start_grid_date(date: NaiveDate) -> NaiveDate {
  let mut first = date.with_day(1).unwrap();

  while first.weekday() != chrono::Weekday::Mon {
    first = first.pred_opt().unwrap_or(first);
  }

  first
}

fn date_to_idx(start: NaiveDate, date: NaiveDate) -> usize {
  ((date - start).num_days() as usize).clamp(0, 41)
}

