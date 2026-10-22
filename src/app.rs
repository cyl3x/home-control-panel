use crate::config::Config;
use crate::messaging;
use crate::messaging::{AppMessage, EnergyMessage, HeatingMessage, Page};
use crate::prelude::*;
use crate::widgets::calendar::CalendarWidget;
use crate::widgets::energy::EnergyWidget;
use crate::widgets::grafana::GrafanaWidget;
use crate::widgets::heating::HeatingWidget;
use crate::widgets::screensaver::ScreensaverWidget;
use crate::widgets::summary::SummaryWidget;
use crate::widgets::switcher::SwitcherWidget;
use crate::widgets::video::Video;

pub struct App {
    #[allow(unused)]
    window: gtk::ApplicationWindow,
    calendar: CalendarWidget,
    video: Video,
    screensaver: ScreensaverWidget,
    grafana: GrafanaWidget,
    heating: Option<HeatingWidget>,
    energy: Option<EnergyWidget>,
    summary: SummaryWidget,
    switcher: SwitcherWidget,
}

impl App {
    pub fn new(app: &gtk::Application, config: &Config) -> Self {
        let calendar = CalendarWidget::new(config);
        let video = Video::new(config);
        let screensaver = ScreensaverWidget::new(config, calendar.upcoming());
        let grafana = GrafanaWidget::new(config);
        let heating = config.myvaillant.as_ref().map(HeatingWidget::new);
        let energy = config.enviiq.as_ref().map(EnergyWidget::new);

        // Below the camera, energy first as it changes more often
        let summary = SummaryWidget::new(
            &[
                energy.is_some().then_some(Page::Energy),
                heating.is_some().then_some(Page::Heating),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
        );

        if let Some(heating) = &heating {
            summary.set(Page::Heating, &heating.summary(), heating.last_success());
        }

        let side = gtk::Box::new(gtk::Orientation::Vertical, 0);
        side.append(video.widget());
        side.append(summary.widget());

        let paned = gtk::Paned::new(gtk::Orientation::Horizontal);
        paned.add_css_class("paned");
        paned.set_expand(true);
        paned.set_start_child(Some(calendar.widget()));
        paned.set_end_child(Some(&side));

        calendar.widget().set_width_request(100);
        side.set_width_request(800);

        let stack = gtk::Stack::new();
        stack.set_transition_type(gtk::StackTransitionType::SlideLeftRight);
        stack.set_transition_duration(300);
        stack.add_titled(&paned, Some("calendar"), "Kalender");
        if !config.grafana.panels.is_empty() {
            stack.add_titled(grafana.widget(), Some("grafana"), "Grafana");
        }
        if let Some(heating) = &heating {
            stack.add_titled(heating.widget(), Some("heating"), "Heizung");
        }
        if let Some(energy) = &energy {
            stack.add_titled(energy.widget(), Some("energy"), "Energie");
        }

        let switcher = SwitcherWidget::new(
            &stack,
            &[
                (Page::Calendar, "calendar"),
                (Page::Heating, "heating"),
                (Page::Energy, "energy"),
            ],
        );

        let view = gtk::Box::new(gtk::Orientation::Vertical, 0);
        view.set_expand(true);
        if stack.pages().n_items() <= 1 {
            view.add_css_class("no-stackswitcher");
        } else {
            view.append(switcher.widget());
        }
        view.append(&stack);

        let overlay = gtk::Overlay::new();
        overlay.set_expand(true);
        overlay.set_child(Some(&view));
        overlay.add_overlay(screensaver.widget());

        let controller = gtk::GestureClick::new();
        controller.connect_pressed(move |controller, _, _, _| {
            if controller.current_button() == gtk::gdk::BUTTON_PRIMARY {
                messaging::send_message(messaging::ScreensaverMessage::Reset);
            }
        });

        let window = gtk::ApplicationWindow::new(app);
        window.set_default_width(900);
        window.set_default_height(600);
        window.set_child(Some(&overlay));
        window.add_controller(controller);
        window.present();

        Self {
            window,
            calendar,
            video,
            screensaver,
            grafana,
            heating,
            energy,
            summary,
            switcher,
        }
    }

    pub fn update(&mut self, message: AppMessage) {
        log::debug!("App received message: {:?}", message);

        match message {
            AppMessage::Calendar(message) => self.calendar.update(message),
            AppMessage::Video(message) => self.video.update(message),
            AppMessage::Screensaver(message) => self.screensaver.update(message),
            AppMessage::Grafana(message) => self.grafana.update(message),
            AppMessage::Heating(message) => {
                if let Some(heating) = &mut self.heating {
                    let fetched = matches!(message, HeatingMessage::Update(_));
                    heating.update(message);

                    if fetched {
                        self.summary
                            .set(Page::Heating, &heating.summary(), heating.last_success());
                    }
                }
            }
            AppMessage::Energy(message) => {
                if let Some(energy) = &mut self.energy {
                    let fetched = matches!(message, EnergyMessage::Update(_));
                    energy.update(message);

                    if fetched {
                        self.summary
                            .set(Page::Energy, &energy.summary(), energy.last_success());
                    }
                }
            }
            AppMessage::Switcher(message) => self.switcher.update(message),
        }
    }
}
