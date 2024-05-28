use chrono::NaiveDate;
use gtk::prelude::*;
use relm4::prelude::*;
use url::Url;

use crate::calendar::caldav::Credentials;
use crate::calendar::{caldav, CalendarService, GridService, GRID_ROWS};
use crate::components::calendar_row;
use crate::icalendar::EventChange;


#[derive(Debug)]
pub struct Widget {
  grid_service: GridService,
  calendar_manager: CalendarService,
  event_rows: [Controller<calendar_row::Widget>; GRID_ROWS],
}

#[derive(Debug, Clone)]
pub enum Input {
  NextMonth,
  PreviousMonth,
  DayClicked(NaiveDate),
  RefreshGrid,
  Sync,
}

#[derive(Debug, Clone)]
pub enum Output {
  CalDavError(caldav::Error),
}

#[relm4::component(pub)]
impl Component for Widget {
  type Init = (Credentials, Url);
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
            // set_row_homogeneous: true,
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
          #[watch] set_text: &model.grid_service.current().format_localized("%B %Y", chrono::Locale::de_DE).to_string(),
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
        self.grid_service.next_month();
        sender.input(Input::RefreshGrid);
      }
      Input::PreviousMonth => {
        self.grid_service.prev_month();
        sender.input(Input::RefreshGrid);
      }
      Input::DayClicked(date) => {
        let prev_date = self.grid_service.current();
        let prev_idx = self.grid_service.current_idx();

        let clicked_row_idx = self.grid_service.row_idx(date);
        let prev_row_idx = self.grid_service.current_row_idx();

        if self.grid_service.set_date(date).is_some() {
          sender.input(Input::RefreshGrid);
        } else {
          self.event_rows[prev_row_idx].emit(calendar_row::Input::SelectDayLabel(
            prev_idx,
            prev_date,
            self.grid_service.current(),
          ));
          self.event_rows[clicked_row_idx].emit(calendar_row::Input::SelectDayLabel(
            self.grid_service.current_idx(),
            date,
            self.grid_service.current(),
          ));
        }
      }
      Input::Sync => {
        let changeset = match self.calendar_manager.sync(self.grid_service.current()) {
          Ok(changeset) => changeset,
          Err(error) => {
            sender.output(Output::CalDavError(error)).unwrap();

            return;
          },
        };
        
        for event_changes in CalendarService::generate_grid_from_changeset(self.grid_service.start_end(), &changeset) {
          for event_change in event_changes {
            for idx in self.grid_service.intersecting_rows(event_change.start_date(), event_change.end_date()) {
              self.event_rows[idx].emit(match event_change {
                EventChange::Added(event) => calendar_row::Input::Add((*event).clone()),
                EventChange::Changed(event) => calendar_row::Input::Update((*event).clone()),
                EventChange::Removed(event) => calendar_row::Input::Remove(event.uid),
              });
            }
          }
        }
      }
      Input::RefreshGrid => {
        let date = self.grid_service.current();

        for row_idx in 0..GRID_ROWS {
          self.event_rows[row_idx].emit(calendar_row::Input::UpdateDayLabels(date, self.grid_service.row(row_idx)));
        }

        for row in &self.event_rows {
          row.emit(calendar_row::Input::Reset);
        }
    
        for events in self.calendar_manager.generate_grid(self.grid_service.start_end()) {
          for event in events {
            for idx in self.grid_service.intersecting_rows(event.start_date(), event.end_date()) {
              self.event_rows[idx].emit(calendar_row::Input::Add((*event).clone()));
            }
          }
        }
      }
    }
  }

  fn init(
    (credendials, url): Self::Init,
    root: Self::Root,
    sender: ComponentSender<Self>,
  ) -> ComponentParts<Self> {
    let grid_service = GridService::new(chrono::Utc::now().naive_utc().date());

    let event_rows = core::array::from_fn(|row_idx| {
      calendar_row::Widget::builder()
        .launch(grid_service.row(row_idx))
        .forward(sender.input_sender(), |output| match output {
          calendar_row::Output::Clicked(date) => Input::DayClicked(date),
        })
    });

    let model = Self {
      grid_service,
      calendar_manager: CalendarService::new(credendials, url),
      event_rows,
    };

    let widgets = view_output!();

    for (row_idx, event_row) in model.event_rows.iter().enumerate() {
      event_row.emit(calendar_row::Input::UpdateDayLabels(model.grid_service.current(), model.grid_service.row(row_idx)));
      widgets.calendar_grid.attach(event_row.widget(), 0, (row_idx * 2 + 1) as i32, 7, 1);
    }

    sender.input(Input::Sync);

    gtk::glib::timeout_add_seconds(30, gtk::glib::clone!(@strong sender => move || {
      println!("Syncing calendar");
      sender.input(Input::Sync);

      gtk::glib::ControlFlow::Continue
    }));
    
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