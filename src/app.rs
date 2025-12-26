use crate::config::Config;
use crate::messaging;
use crate::messaging::AppMessage;
use crate::prelude::*;
use crate::widgets::calendar::CalendarWidget;
use crate::widgets::grafana::GrafanaWidget;
use crate::widgets::screensaver::ScreensaverWidget;
use crate::widgets::video::Video;

pub struct App {
    window: gtk::ApplicationWindow,
    calendar: CalendarWidget,
    video: Video,
    screensaver: ScreensaverWidget,
    grafana: GrafanaWidget,
}

impl App {
    pub fn new(app: &gtk::Application, config: &Config) -> Self {
        let calendar = CalendarWidget::new(config);
        let video = Video::new(config);
        let screensaver = ScreensaverWidget::new(config, calendar.upcoming());
        let grafana = GrafanaWidget::new(config);

        let paned = gtk::Paned::new(gtk::Orientation::Horizontal);
        paned.add_css_class("paned");
        paned.set_expand(true);
        paned.set_start_child(Some(calendar.widget()));
        paned.set_end_child(Some(video.widget()));

        calendar.widget().set_width_request(100);
        video.widget().set_width_request(800);

        let stack = gtk::Stack::new();
        stack.set_transition_type(gtk::StackTransitionType::SlideLeftRight);
        stack.set_transition_duration(300);
        stack.add_titled(&paned, Some("calendar"), "Kalendar");
        stack.add_titled(grafana.widget(), Some("grafana"), "Grafana");

        let stack_switcher = gtk::StackSwitcher::new();
        stack_switcher.set_hexpand(true);
        stack_switcher.set_stack(Some(&stack));

        let view = gtk::Box::new(gtk::Orientation::Vertical, 0);
        view.set_expand(true);
        view.append(&stack_switcher);
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
        }
    }

    pub fn update(&mut self, message: AppMessage) {
        match message {
            AppMessage::Calendar(message) => self.calendar.update(message),
            AppMessage::Video(message) => self.video.update(message),
            AppMessage::Screensaver(message) => self.screensaver.update(message),
            AppMessage::Grafana(message) => self.grafana.update(message),
        };
    }
}
