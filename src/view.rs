use chrono::NaiveDate;
use relm4::factory::FactoryHashMap;
use relm4::{Component, ComponentController, ComponentParts, ComponentSender, Controller, FactorySender, RelmApp, RelmWidgetExt as _, WidgetTemplate};
use relm4::gtk::prelude::*;
use uuid::Uuid;

use crate::calendar::caldav::{self, Credentials};
use crate::calendar::{CalendarService, GridService, GRID_ROWS};
use crate::components::{calendar_row, calendar_selection, event_list_day};
use crate::config::Config;
use crate::icalendar::CalendarMap;

#[derive(Debug)]
pub enum Input {
  MonthCalendarPrevious,
  MonthCalendarNext,
  MonthCalendarRowClicked(f64),
  MonthCalendarDayClicked(NaiveDate),
  CalendarSelectionClicked(Uuid, bool),
  CalendarSync,
  VideoPrevious,
  VideoNext,
  TickNow,
}

#[derive(Debug)]
pub enum Command {
  MonthCalendarRebuild,
  DayCalendarRebuild,
  Sync(CalendarMap),
  CalDavError(caldav::Error),
}

pub struct App {
  config: Config,
  grid_service: GridService,
  calendar_manager: CalendarService,
  calendar_selection: FactoryHashMap<Uuid, calendar_selection::Widget>,
  day_calendar_days: FactoryHashMap<NaiveDate, event_list_day::Widget>,
}

#[relm4::component(pub)]
impl Component for App {
  type Init = Config;
  type Input = Input;
  type Output = ();
  type CommandOutput = Command;

  fn init(
    config: Self::Init,
    root: Self::Root,
    sender: ComponentSender<Self>,
  ) -> ComponentParts<Self> {
    let model = Self {
      calendar_manager: CalendarService::new(Credentials::from(&config), &config.ical.url),
      config,
      grid_service: GridService::new(chrono::Utc::now().date_naive()),
      day_calendar_days: FactoryHashMap::builder().launch(event_list_day::create_parent()).detach(),
      calendar_selection: FactoryHashMap::builder().launch(calendar_selection::create_parent()).forward(sender.input_sender(), |output| match output {
        calendar_selection::Output::Clicked(uid, is_active) => Input::CalendarSelectionClicked(uid, is_active),
      }),
    };

    let widgets = view_output!();

    ComponentParts { model, widgets }
  }

  view! {
    #[name(window)]
    gtk::Window {
      set_default_size: (600, 300),

      #[name(window_overlay)]
      gtk::Overlay {

        #[name(status_bar)]
        add_overlay = &gtk::Statusbar {
          set_hexpand: true,
          set_vexpand: true,
          set_valign: gtk::Align::End,
          set_halign: gtk::Align::End,
        },

        #[name(calendar_and_cams_paned)]
        gtk::Paned {
          set_orientation: gtk::Orientation::Horizontal,
          set_vexpand: true,
          set_hexpand: true,
          set_wide_handle: true,

          #[wrap(Some)]
          #[name(calendar_box)]
          set_start_child = &gtk::Box {
            inline_css: "min-width: 400px;",
            set_orientation: gtk::Orientation::Vertical,
            set_hexpand: true,
            set_vexpand: true,

            gtk::Paned {
              set_orientation: gtk::Orientation::Vertical,
              set_hexpand: true,
              set_vexpand: true,
              set_wide_handle: true,

              #[wrap(Some)]
              #[name(month_calendar_box)]
              set_start_child = &gtk::Box {
                add_css_class: "calendar",
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 16,
                set_margin_top: 16,
                set_margin_bottom: 16,
                set_margin_start: 16,
                set_margin_end: 16,

                gtk::Box {
                  #[name(month_calendar_prev_button)]
                  gtk::Button {
                    set_icon_name: "pan-start-symbolic",
                    set_size_request: (52, 52),
                    set_halign: gtk::Align::Start,

                    connect_clicked => Input::MonthCalendarPrevious,
                  },

                  #[name(month_calendar_label)]
                  gtk::Label {
                    inline_css: "font-size: 24px; font-weight: semi-bold;",
                    set_text: "<-------->",
                    set_hexpand: true,
                    set_halign: gtk::Align::Fill,
                  },

                  #[name(month_calendar_next_button)]
                  gtk::Button {
                    set_icon_name: "pan-end-symbolic",
                    set_size_request: (52, 52),
                    set_halign: gtk::Align::Start,

                    connect_clicked => Input::MonthCalendarNext,
                  },
                },

                #[name(month_calendar_grid_window)]
                gtk::ScrolledWindow {
                  set_hscrollbar_policy: gtk::PolicyType::Never,
                  set_vscrollbar_policy: gtk::PolicyType::Automatic,

                  #[name(month_calendar_grid)]
                  gtk::Grid {
                    add_css_class: "calendar-grid",

                    set_valign: gtk::Align::Start,
                    set_hexpand: true,
                    set_vexpand: true,
                    set_column_homogeneous: true,

                    attach[0, 0, 1, 1] = &gtk::Label { inline_css: "font-size: 20px; font-weight: bold;", set_text: "Mo", },
                    attach[1, 0, 1, 1] = &gtk::Label { inline_css: "font-size: 20px; font-weight: bold;", set_text: "Di", },
                    attach[2, 0, 1, 1] = &gtk::Label { inline_css: "font-size: 20px; font-weight: bold;", set_text: "Mi", },
                    attach[3, 0, 1, 1] = &gtk::Label { inline_css: "font-size: 20px; font-weight: bold;", set_text: "Do", },
                    attach[4, 0, 1, 1] = &gtk::Label { inline_css: "font-size: 20px; font-weight: bold;", set_text: "Fr", },
                    attach[5, 0, 1, 1] = &gtk::Label { inline_css: "font-size: 20px; font-weight: bold;", set_text: "Sa", },
                    attach[6, 0, 1, 1] = &gtk::Label { inline_css: "font-size: 20px; font-weight: bold;", set_text: "So", },

                    #[template]
                    #[name(month_calendar_row_0)]
                    attach[0, 1, 7, 1] = &MonthCalendarRow {
                      add_controller = gtk::GestureClick {
                        connect_pressed[sender] => move |controller, _, x, _| {
                          if controller.current_button() == gtk::gdk::BUTTON_PRIMARY {
                            sender.input(Input::MonthCalendarRowClicked(x));
                          }
                        },
                      },
                    },

                    #[template]
                    #[name(month_calendar_row_1)]
                    attach[0, 2, 7, 1] = &MonthCalendarRow {
                      add_controller = gtk::GestureClick {
                        connect_pressed[sender] => move |controller, _, x, _| {
                          if controller.current_button() == gtk::gdk::BUTTON_PRIMARY {
                            sender.input(Input::MonthCalendarRowClicked(x));
                          }
                        },
                      },
                    },

                    #[template]
                    #[name(month_calendar_row_2)]
                    attach[0, 3, 7, 1] = &MonthCalendarRow {
                      add_controller = gtk::GestureClick {
                        connect_pressed[sender] => move |controller, _, x, _| {
                          if controller.current_button() == gtk::gdk::BUTTON_PRIMARY {
                            sender.input(Input::MonthCalendarRowClicked(x));
                          }
                        },
                      },
                    },

                    #[template]
                    #[name(month_calendar_row_3)]
                    attach[0, 4, 7, 1] = &MonthCalendarRow {
                      add_controller = gtk::GestureClick {
                        connect_pressed[sender] => move |controller, _, x, _| {
                          if controller.current_button() == gtk::gdk::BUTTON_PRIMARY {
                            sender.input(Input::MonthCalendarRowClicked(x));
                          }
                        },
                      },
                    },

                    #[template]
                    #[name(month_calendar_row_4)]
                    attach[0, 5, 7, 1] = &MonthCalendarRow {
                      add_controller = gtk::GestureClick {
                        connect_pressed[sender] => move |controller, _, x, _| {
                          if controller.current_button() == gtk::gdk::BUTTON_PRIMARY {
                            sender.input(Input::MonthCalendarRowClicked(x));
                          }
                        },
                      },
                    },

                    #[template]
                    #[name(month_calendar_row_5)]
                    attach[0, 6, 7, 1] = &MonthCalendarRow {
                      add_controller = gtk::GestureClick {
                        connect_pressed[sender] => move |controller, _, x, _| {
                          if controller.current_button() == gtk::gdk::BUTTON_PRIMARY {
                            sender.input(Input::MonthCalendarRowClicked(x));
                          }
                        },
                      },
                    },
                  },
                }
              },

              #[wrap(Some)]
              #[name(day_calendar_window)]
              set_end_child = &gtk::ScrolledWindow {
                set_hscrollbar_policy: gtk::PolicyType::Never,
                set_vscrollbar_policy: gtk::PolicyType::Automatic,
                set_child: Some(model.day_calendar_days.widget()),
              },
            },

            #[name(calendar_selection)]
            gtk::ScrolledWindow {
              set_hscrollbar_policy: gtk::PolicyType::Automatic,
              set_vscrollbar_policy: gtk::PolicyType::Never,
              set_child: Some(model.calendar_selection.widget()),
            },
          },

          #[wrap(Some)]
          #[name(cams_box)]
          set_end_child = &gtk::CenterBox {
            inline_css: "background-color: #000000;",

            set_orientation: gtk::Orientation::Vertical,
            set_vexpand: true,
            set_hexpand: true,
            set_size_request: (600, -1),

            #[wrap(Some)]
            #[name(video)]
            set_center_widget = &clapper_gtk::Video {
              add_fading_overlay = &gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_hexpand: true,
                set_valign: gtk::Align::Start,
                set_halign: gtk::Align::Fill,
                set_margin_all: 16,
                set_spacing: 8,

                gtk::Button {
                  set_css_classes: &["osd"],
                  set_icon_name: "pan-start-symbolic",
                  set_size_request: (52, 52),
                  set_halign: gtk::Align::Start,

                  connect_clicked => Input::VideoPrevious,
                },

                #[name(video_drop_down)]
                gtk::Box {
                  set_css_classes: &["osd"],
                  set_hexpand: true,
                  set_halign: gtk::Align::Fill,

                  gtk::DropDown {
                    set_hexpand: true,
                    set_halign: gtk::Align::Fill,
                  },
                },

                gtk::Button {
                  set_css_classes: &["osd"],
                  set_icon_name: "pan-end-symbolic",
                  set_size_request: (52, 52),
                  set_halign: gtk::Align::Start,

                  connect_clicked => Input::VideoNext,
                },
              }
            },
          },
        }
      }
    }
  }

  fn update_with_view(&mut self, widgets: &mut Self::Widgets, input: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
    match input {
      Input::MonthCalendarPrevious => {
        self.grid_service.prev_month();
        sender.command_sender().send(Command::MonthCalendarRebuild).unwrap();
      }
      Input::MonthCalendarNext => {
        self.grid_service.next_month();
        sender.command_sender().send(Command::MonthCalendarRebuild).unwrap();
      }
      _ => unimplemented!(),
    }
  }

  fn update_cmd(
    &mut self,
    command: Self::CommandOutput,
    sender: ComponentSender<Self>,
    _root: &Self::Root,
  ) {
    match command {
      Command::CalDavError(error) => (),
      Command::Sync(new_calendar_map) => {
      }
      Command::MonthCalendarRebuild => {

      }
      Command::DayCalendarRebuild => {
      }
    }
  }
}

#[relm4::widget_template(pub)]
impl WidgetTemplate for MonthCalendarRow {
  view! {
    gtk::Grid {
      inline_css: "min-height: 100px; padding: 4px 0px;",
      set_hexpand: true,
      set_vexpand: true,
      set_column_homogeneous: true,
      set_row_spacing: 4,
    }
  }
}
