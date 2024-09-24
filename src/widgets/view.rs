use chrono::{NaiveDate, NaiveDateTime};
use gtk::prelude::*;
use relm4::prelude::*;
use uuid::Uuid;

use crate::calendar::caldav::Credentials;
use crate::calendar::{caldav, CalendarService, Event};
use crate::config::{self, Config};
use crate::calendar::CalendarMap;

use super::{calendar_selection, day_calendar, month_calendar, video};

#[derive(Debug)]
pub struct Widget {
  calendar_manager: CalendarService,
  month_calendar: Controller<month_calendar::Widget>,
  day_calendar: Controller<day_calendar::Widget>,
  calendar_selection: Controller<calendar_selection::Widget>,
  video: Controller<video::Widget>,
  calendar_configs: config::Calendars,
}

#[derive(Debug, Clone)]
pub enum Input {
  Tick(NaiveDateTime),
  Sync,
  MonthCalendarSelected(NaiveDate),
  CalendarSelectionClicked(Uuid, bool),
  BuildMonthCalendar(NaiveDate, NaiveDate),
  BuildDayCalendar(NaiveDate),
  BuildCalendarSelection,
}

#[derive(Debug, Clone)]
pub enum Output {
  CalDavError(caldav::Error),
}


#[derive(Debug)]
pub enum Command {
  Sync(CalendarMap),
  CalDavError(caldav::Error),
}

#[relm4::component(pub)]
impl Component for Widget {
  type Init = Config;
  type Input = Input;
  type Output = Output;
  type CommandOutput = Command;

  view! {
    gtk::Paned {
      set_orientation: gtk::Orientation::Horizontal,
      set_hexpand: true,
      set_vexpand: true,
      set_wide_handle: true,

      #[wrap(Some)]
      set_start_child = left = &gtk::Box {
        set_orientation: gtk::Orientation::Vertical,
        append: model.month_calendar.widget(),
        append: model.calendar_selection.widget(),
        append: model.day_calendar.widget(),
      },

      #[wrap(Some)]
      set_end_child = right = &gtk::Paned {
        set_orientation: gtk::Orientation::Vertical,
        set_hexpand: true,
        set_vexpand: true,
        set_wide_handle: true,
        set_size_request: (800, -1),
        set_shrink_start_child: false,

        set_start_child: Some(model.video.widget()),
        #[wrap(Some)] set_end_child = &gtk::Box {}
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
      Input::Sync => {
        let client = self.calendar_manager.client().clone();
        let date = chrono::Utc::now().date_naive();

        sender.spawn_oneshot_command(move || {
          CalendarService::fetch(client, date).map_or_else(
            Command::CalDavError,
            Command::Sync,
          )
        });
      }
      Input::Tick(now) => {
        self.month_calendar.emit(month_calendar::Input::Tick(now));
        self.day_calendar.emit(day_calendar::Input::Tick(now));
      }
      Input::BuildMonthCalendar(start, end) => {
        log::debug!("Building month calendar from {} to {}", start, end);

        for event in self.calendar_manager.events_filtered() {
          if is_included(event, &self.calendar_configs.month) && event.is_between_dates(start, end) {
            self.month_calendar.emit(month_calendar::Input::Add(Box::new(event.clone())));
          }
        }
      }
      Input::BuildDayCalendar(date) => {
        log::debug!("Building day calendar for {}", date);

        for event in self.calendar_manager.events_filtered() {
          if is_included(event, &self.calendar_configs.day) && event.is_between_dates(date, date) {
            self.day_calendar.emit(day_calendar::Input::Add(Box::new(event.clone())));
          }
        }
      }
      Input::BuildCalendarSelection => {
        for calendar in self.calendar_manager.calendars() {
          self.calendar_selection.emit(calendar_selection::Input::Add(Box::new(calendar.clone())));
        }
      }
      Input::MonthCalendarSelected(date) => {
        self.day_calendar.emit(day_calendar::Input::SetDay(date));
      }
      Input::CalendarSelectionClicked(uid, is_active) => {
        self.calendar_manager.toggle_calendar_filter(uid, is_active);

        self.month_calendar.emit(month_calendar::Input::Reset);
        self.day_calendar.emit(day_calendar::Input::Reset);
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
        let mut has_changes = false;

        // consume the iterator to apply all changes
        for _ in self.calendar_manager.apply_map(new_calendar_map) {
          has_changes = true;
        };

        if has_changes {
          self.month_calendar.emit(month_calendar::Input::Reset);
          self.day_calendar.emit(day_calendar::Input::Reset);
          self.calendar_selection.emit(calendar_selection::Input::Reset);
        }
      }
    }
  }

  fn init(
    config: Self::Init,
    root: Self::Root,
    sender: ComponentSender<Self>,
  ) -> ComponentParts<Self> {
    let date = chrono::Utc::now().date_naive();

    let model = Self {
      calendar_configs: config.calendar,
      calendar_manager: CalendarService::new(Credentials::from(&config.ical), config.ical.url),
      month_calendar: month_calendar::Widget::builder().launch(date).forward(sender.input_sender(), |output| match output {
        month_calendar::Output::RequestEvents(start, end) => Input::BuildMonthCalendar(start, end),
        month_calendar::Output::Selected(date) => Input::MonthCalendarSelected(date),
      }),
      day_calendar: day_calendar::Widget::builder().launch(date).forward(sender.input_sender(), |output| match output {
        day_calendar::Output::RequestEvents(date) => Input::BuildDayCalendar(date),
      }),
      calendar_selection: calendar_selection::Widget::builder().launch(()).forward(sender.input_sender(), |output| match output {
        calendar_selection::Output::RequestCalendars => Input::BuildCalendarSelection,
        calendar_selection::Output::Clicked(uid, is_active) => Input::CalendarSelectionClicked(uid, is_active),
      }),
      video: video::Widget::builder().launch(config.videos).detach(),
    };

    let widgets = view_output!();

    model.video.widget().set_size_request(-1, 400);

    sender.input(Input::Sync);

    gtk::glib::timeout_add_seconds(5*60, gtk::glib::clone!(@strong sender => move || {
      log::info!("Syncing calendar");
      sender.input(Input::Sync);

      gtk::glib::ControlFlow::Continue
    }));

    gtk::glib::timeout_add_seconds(60, gtk::glib::clone!(@strong sender => move || {
      log::debug!("Calendar: Next tick (60s)");
      sender.input(Input::Tick(chrono::Utc::now().naive_utc()));

      gtk::glib::ControlFlow::Continue
    }));

    ComponentParts { model, widgets }
  }
}

fn is_included(event: &Event, config: &Option<config::CalendarConfig>) -> bool {
  if let Some(config) = config {
    if !config.include.is_empty() && config.include.contains(&event.calendar_uid) {
      return true;
    }

    if !config.exclude.is_empty() && !config.exclude.contains(&event.calendar_uid) {
      return true;
    }

    false
  } else {
    true
  }
}
