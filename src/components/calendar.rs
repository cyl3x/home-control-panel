use chrono::NaiveDate;
use gtk::prelude::*;
use relm4::prelude::*;

use crate::calendar::CalDavProvider;
use crate::components::calendar_row;
use crate::calendar::{GRID_COLS, GRID_ROWS};


#[derive(Debug)]
pub struct Widget {
  provider: CalDavProvider,
  event_rows: [Controller<calendar_row::Widget>; GRID_ROWS],
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Input {
  NextMonth,
  PreviousMonth,
  DayClicked(usize),
  RefreshGrid(NaiveDate),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Output {
}

#[relm4::component(pub)]
impl Component for Widget {
  type Init = CalDavProvider;
  type Input = Input;
  type Output = Output;
  type CommandOutput = ();

  view! {
    #[root]
    gtk::Paned {
      set_orientation: gtk::Orientation::Vertical,
      set_hexpand: true,
      set_vexpand: true,
      set_wide_handle: true,

      #[template]
      #[wrap(Some)]
      set_start_child = &CalendarGridWidget {
        #[template_child] root {
          add_css_class: "calendar",
        },
        #[template_child] grid_stack {
          #[name(calendar_grid)]
          gtk::Grid {
            add_css_class: "calendar-grid",

            set_halign: gtk::Align::Fill,
            set_valign: gtk::Align::Start,
            set_hexpand: true,
            set_vexpand: true,
            set_row_homogeneous: true,
            set_column_homogeneous: true,
            set_row_spacing: 4,

            attach[0, 0, 1, 1] = &gtk::Label { inline_css: "font-size: 20px; font-weight: bold;", set_text: "Mo", },
            attach[1, 0, 1, 1] = &gtk::Label { inline_css: "font-size: 20px; font-weight: bold;", set_text: "Di", },
            attach[2, 0, 1, 1] = &gtk::Label { inline_css: "font-size: 20px; font-weight: bold;", set_text: "Mi", },
            attach[3, 0, 1, 1] = &gtk::Label { inline_css: "font-size: 20px; font-weight: bold;", set_text: "Do", },
            attach[4, 0, 1, 1] = &gtk::Label { inline_css: "font-size: 20px; font-weight: bold;", set_text: "Fr", },
            attach[5, 0, 1, 1] = &gtk::Label { inline_css: "font-size: 20px; font-weight: bold;", set_text: "Sa", },
            attach[6, 0, 1, 1] = &gtk::Label { inline_css: "font-size: 20px; font-weight: bold;", set_text: "So", },
          },
        },

        #[template_child] month_label {
          #[watch] set_text: &model.provider.selected().format_localized("%B %Y", chrono::Locale::de_DE).to_string(),
        },

        #[template_child] prev_button {
          connect_clicked => Input::PreviousMonth,
        },

        #[template_child] next_button {
          connect_clicked => Input::NextMonth,
        },
      },

      #[template]
      #[wrap(Some)]
      #[name(event_list)]
      set_end_child = &CalendarEventListWidget {},
    },
  }

  fn update(
    &mut self,
    input: Self::Input,
    sender: ComponentSender<Self>,
    _root: &Self::Root,
  ) {
    match input {
      Input::NextMonth => {
        let date = self.provider.next_month();
        sender.input(Input::RefreshGrid(date));
      }
      Input::PreviousMonth => {
        let date = self.provider.prev_month();
        sender.input(Input::RefreshGrid(date));
      }
      Input::DayClicked(clicked_idx) => {
        let prev_idx = self.provider.selected_idx();

        let clicked_row_idx = clicked_idx / GRID_COLS;
        let prev_row_idx = prev_idx / GRID_COLS;

        if self.provider.select_idx(clicked_idx).is_some() {
          sender.input(Input::RefreshGrid(self.provider.selected()));
        } else {
          self.event_rows[prev_row_idx].emit(calendar_row::Input::Select(prev_idx, self.provider.date(prev_idx), self.provider.selected()));
          self.event_rows[clicked_row_idx].emit(calendar_row::Input::Select(clicked_idx, self.provider.date(clicked_idx), self.provider.selected()));
        }
      }
      Input::RefreshGrid(date) => {
        for row_idx in 0..GRID_ROWS {
          self.event_rows[row_idx].emit(calendar_row::Input::UpdateDates(date, self.provider.date_row(row_idx)));
        }

        for row in &self.event_rows {
          row.emit(calendar_row::Input::Reset);
        }
    
        let grid = self.provider.calendar_grid();
    
        for (event_start_idx, events) in grid.into_iter().enumerate() {
          for (event_end_idx, event) in events {
            let row_idx = event_start_idx / GRID_COLS;
            let last_row_idx = event_end_idx / GRID_COLS;
    
            let matched_rows = &self.event_rows[row_idx..=last_row_idx];
            for matched_row in matched_rows {
              matched_row.emit(calendar_row::Input::Add((event_start_idx, event_end_idx, event.clone())));
            }
          }
        }
    
        for row in &self.event_rows {
          row.emit(calendar_row::Input::Finish);
        }
      }
    }
  }

  fn init(
    provider: Self::Init,
    root: Self::Root,
    sender: ComponentSender<Self>,
  ) -> ComponentParts<Self> {
    let selected_date = provider.selected();

    let event_rows = core::array::from_fn(|row_idx| {
      calendar_row::Widget::builder()
      .launch(row_idx * GRID_COLS)
      .forward(sender.input_sender(), |output| match output {
        calendar_row::Output::Clicked(idx) => Input::DayClicked(idx),
      })
    });

    let model = Self {
      provider,
      event_rows,
    };

    let widgets = view_output!();

    for (row_idx, event_row) in model.event_rows.iter().enumerate() {
      event_row.emit(calendar_row::Input::UpdateDates(selected_date, model.provider.date_row(row_idx)));
      widgets.calendar_grid.attach(event_row.widget(), 0, (row_idx * 2 + 1) as i32, 7, 1);
    }

    sender.input(Input::RefreshGrid(model.provider.selected()));
    
    ComponentParts { model, widgets }
  }
}

#[relm4::widget_template(pub)]
impl WidgetTemplate for CalendarEventListWidget {
  view! {
    #[name(root)]
    gtk::Box {
      set_orientation: gtk::Orientation::Vertical,
      set_halign: gtk::Align::Fill,
      set_valign: gtk::Align::Start,
      set_hexpand: true,
      set_spacing: 16,
      set_margin_top: 16,
      set_margin_bottom: 16,
      set_margin_start: 16,
      set_margin_end: 16,
    }
  }
}

#[relm4::widget_template(pub)]
impl WidgetTemplate for CalendarGridWidget {
  view! {
    #[name(root)]
    gtk::Box {
      set_orientation: gtk::Orientation::Vertical,
      set_halign: gtk::Align::Fill,
      set_valign: gtk::Align::Start,
      set_hexpand: true,
      set_spacing: 16,
      set_margin_top: 16,
      set_margin_bottom: 16,
      set_margin_start: 16,
      set_margin_end: 16,

      gtk::Box {
        set_hexpand: true,
        set_halign: gtk::Align::Fill,

        #[name(prev_button)]
        gtk::Button {
          set_icon_name: "pan-start-symbolic",
          set_size_request: (52, 52),
          set_halign: gtk::Align::Start,
        },

        #[name(month_label)]
        gtk::Label {
          inline_css: "font-size: 24px; font-weight: semi-bold;",
          set_text: "Month",
          set_hexpand: true,
          set_halign: gtk::Align::Fill,
        },

        #[name(next_button)]
        gtk::Button {
          set_icon_name: "pan-end-symbolic",
          set_size_request: (52, 52),
          set_halign: gtk::Align::Start,
        },
      },

      #[name(grid_stack)]
      gtk::Stack {

      }
    }
  }
}