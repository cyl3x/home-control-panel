use chrono::{Days, NaiveDate};
use gtk::prelude::*;
use relm4::factory::FactoryHashMap;
use relm4::prelude::*;
use url::Url;
use uuid::Uuid;

use crate::calendar::caldav::Credentials;
use crate::calendar::{caldav, CalendarService, GridService, GRID_ROWS};
use crate::components::calendar_row;
use crate::icalendar::{CalendarMap, CalendarMapChange, Event};

use super::event_list_day::{self, FactoryHashMapExt};
use super::calendar_selection;


#[derive(Debug)]
pub struct Widget {
  grid_service: GridService,
  calendar_manager: CalendarService,
  event_rows: [Controller<calendar_row::Widget>; GRID_ROWS],
  day_list: FactoryHashMap<NaiveDate, event_list_day::Widget>,
  calendar_selection: FactoryHashMap<Uuid, calendar_selection::Widget>,
}

#[derive(Debug, Clone)]
pub enum Input {
  NextMonth,
  PreviousMonth,
  DayClicked(NaiveDate),
  TickNow,
  Sync,
}

#[derive(Debug, Clone)]
pub enum Output {
  CalDavError(caldav::Error),
}


#[derive(Debug)]
pub enum Command {
  RebuildGrid,
  RefreshEventList,
  Sync(CalendarMap),
  CalDavError(caldav::Error),
}

#[relm4::component(pub)]
impl Component for Widget {
  type Init = (Credentials, Url);
  type Input = Input;
  type Output = Output;
  type CommandOutput = Command;

  view! {
    gtk::Box {
      set_orientation: gtk::Orientation::Vertical,
      set_hexpand: true,
      set_vexpand: true,

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
            #[watch] set_text: &model.grid_service.selected().format_localized("%B %Y", chrono::Locale::de_DE).to_string(),
          },

          #[template_child] prev_button {
            connect_clicked => Input::PreviousMonth,
          },

          #[template_child] next_button {
            connect_clicked => Input::NextMonth,
          },
        },

        set_end_child: Some(model.day_list.widget()),
      },

      gtk::ScrolledWindow {
        set_hexpand: true,
        set_hscrollbar_policy: gtk::PolicyType::Automatic,
        set_vscrollbar_policy: gtk::PolicyType::Never,
        set_min_content_height: 10,
        set_max_content_width: 10,
        set_child: Some(model.calendar_selection.widget()),
      },
    },
  }

  fn update(
    &mut self,
    input: Self::Input,
    sender: ComponentSender<Self>,
    _root: &Self::Root,
  ) {
    match input {
      Input::TickNow => {
        let (day, month) = self.grid_service.tick();

        if month {
          sender.command_sender().send(Command::RebuildGrid).unwrap();
        };
        
        if day {
          sender.command_sender().send(Command::RefreshEventList).unwrap();
        } else {
          self.day_list.broadcast(event_list_day::Input::TickNow(self.grid_service.now()));
        }
      }
      Input::NextMonth => {
        self.grid_service.next_month();
        sender.command_sender().send(Command::RebuildGrid).unwrap();
      }
      Input::PreviousMonth => {
        self.grid_service.prev_month();
        sender.command_sender().send(Command::RebuildGrid).unwrap();
      }
      Input::DayClicked(date) => {
        let prev_date = self.grid_service.selected();
        let prev_idx = self.grid_service.selected_idx();

        let clicked_row_idx = self.grid_service.row_idx(date);
        let prev_row_idx = self.grid_service.selected_row_idx();

        if self.grid_service.set_date(date).is_some() {
          sender.command_sender().send(Command::RebuildGrid).unwrap();
        } else {
          self.event_rows[prev_row_idx].state().get_mut().model.select_day_label(
            prev_idx,
            prev_date,
            self.grid_service.selected(),
          );
          self.event_rows[clicked_row_idx].state().get_mut().model.select_day_label(
            self.grid_service.selected_idx(),
            date,
            self.grid_service.selected(),
          );
        }
      }
      Input::Sync => {
        let client = self.calendar_manager.client().clone();
        let date = self.grid_service.selected();

        sender.spawn_oneshot_command(move || {
          CalendarService::fetch(client, date).map_or_else(
            Command::CalDavError,
            Command::Sync,
          )
        });
      }
    }
  }

  fn update_cmd(
    &mut self,
    command: Self::CommandOutput,
    sender: ComponentSender<Self>,
    _root: &Self::Root,
  ) {
    match command {
      Command::CalDavError(error) => sender.output(Output::CalDavError(error)).unwrap(),
      Command::Sync(new_calendar_map) => {
        let day_list_now = self.grid_service.now().date();
        let day_list_later = day_list_now + Days::new(14);

        for (_, _, event_change) in self.calendar_manager.apply_map(new_calendar_map) {
          // Update grid
          if event_change.is_between_dates(self.grid_service.start(), self.grid_service.end()) {
            let range = self.grid_service.intersecting_rows(event_change.start, event_change.end);
            Self::sync_grid(&self.event_rows[range], &event_change);
          }

          // Update event list
          if event_change.is_between_dates(day_list_now, day_list_later) {
            Self::sync_event_list(&mut self.day_list, event_change);
          }
        }

        Self::sync_calendar_selection(&mut self.calendar_selection, &self.calendar_manager);

        self.day_list.resort();
      }
      Command::RebuildGrid => {
        Self::sync_grid_day_labels(&self.event_rows, &self.grid_service);
        Self::reset_event_rows(&self.event_rows);

        for event in self.calendar_manager.events() {
          if event.is_between_dates(self.grid_service.start(), self.grid_service.end()) {
            Self::add_event(&self.event_rows, event);
          }
        }
      }
      Command::RefreshEventList => {
        let now = self.grid_service.now().date();
        let later = now + Days::new(14);

        for event in self.calendar_manager.events() {
          if event.is_between_dates(now, later) {
            self.day_list.insert(event.start_date(), event.clone());

            continue;
          }

          if self.day_list.get(&event.start_date()).is_some() {
            let is_empty = self.day_list.get_mut(&event.start_date()).unwrap().remove(&event.uid); 

            if is_empty {
              self.day_list.remove(&event.start_date());
            }
          }
        }
        
        self.day_list.tick(self.grid_service.now());
        self.day_list.resort();
      }
    }
  }

  fn init(
    (credendials, url): Self::Init,
    root: Self::Root,
    sender: ComponentSender<Self>,
  ) -> ComponentParts<Self> {
    let grid_service = GridService::new(chrono::Utc::now().date_naive());

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
      day_list: FactoryHashMap::builder()
        .launch(event_list_day::create_parent())
        .detach(),
      calendar_selection: FactoryHashMap::builder()
        .launch(calendar_selection::create_parent())
        .detach(),
    };

    let widgets = view_output!();

    for (row_idx, event_row) in model.event_rows.iter().enumerate() {
      event_row.state().get_mut().model.update_day_labels(model.grid_service.selected(), model.grid_service.row(row_idx));
      widgets.calendar_grid.attach(event_row.widget(), 0, (row_idx * 2 + 1) as i32, 7, 1);
    }

    sender.input(Input::Sync);

    gtk::glib::timeout_add_seconds(5*60, gtk::glib::clone!(@strong sender => move || {
      println!("Syncing calendar");
      sender.input(Input::Sync);

      gtk::glib::ControlFlow::Continue
    }));

    gtk::glib::timeout_add_seconds(60, gtk::glib::clone!(@strong sender => move || {
      log::debug!("Calendar: Next tick (60s)");
      sender.input(Input::TickNow);

      gtk::glib::ControlFlow::Continue
    }));
    
    ComponentParts { model, widgets }
  }
}

impl Widget {
  fn sync_grid(rows: &[Controller<calendar_row::Widget>], event_change: &CalendarMapChange) {
    match event_change {
      CalendarMapChange::Added(event) | CalendarMapChange::Changed(event) => Self::add_event(rows, event),
      CalendarMapChange::Removed(event) => Self::remove_event(rows, event),
    }
  }

  fn sync_event_list(day_list: &mut FactoryHashMap<NaiveDate, event_list_day::Widget>, event_change: CalendarMapChange) {
    let start_date = event_change.start_date();
    if day_list.get(&start_date).is_some() {
      let is_empty = day_list.get_mut(&start_date).unwrap().apply(&event_change); 

      if is_empty {
        day_list.remove(&start_date);
      }
    } else if !event_change.is_removed() {
      day_list.insert(start_date, event_change.into_inner());
    }
  }

  fn sync_grid_day_labels(event_rows: &[Controller<calendar_row::Widget>], grid_service: &GridService) {
    let date = grid_service.selected();

    for (row_idx, row) in event_rows.iter().enumerate() {
      row.state().get_mut().model.update_day_labels(date, grid_service.row(row_idx));
    }
  }

  fn sync_calendar_selection(calendar_selection: &mut FactoryHashMap<Uuid, calendar_selection::Widget>, calendar_manager: &CalendarService) {
    let old_uids = calendar_selection.keys().cloned().collect::<Vec<_>>();
    for uid in old_uids {
      if !calendar_manager.calendar_map().contains_key(&uid) {
        calendar_selection.remove(&uid);
      }
    }

    for calendar in calendar_manager.calendars() {
      calendar_selection.insert(calendar.uid, calendar.clone());
    }
  }

  fn add_event(event_rows: &[Controller<calendar_row::Widget>], event: &Event) {
    for row in event_rows {
      row.state().get_mut().model.add_event(row.widget(), event);
    }
  }

  fn remove_event(event_rows: &[Controller<calendar_row::Widget>], event: &Event) {
    for row in event_rows {
      row.state().get_mut().model.remove_event(&event.uid);
    }
  }

  fn reset_event_rows(event_rows: &[Controller<calendar_row::Widget>]) {
    for row in event_rows {
      row.state().get_mut().model.reset();
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