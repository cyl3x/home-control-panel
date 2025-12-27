use std::rc::Rc;
use std::sync::Mutex;

use gtk::{Paned, glib, prelude::*};
use gtk::{Application, ApplicationWindow};

use crate::{app, messaging};
use crate::calendar::CalendarMap;
use crate::config::Config;
use crate::messaging::{AppMessage, AppReceiver, AppSender};
use crate::widgets::calendar::Calendar;
use crate::widgets::video::Video;

pub struct App {
    window: ApplicationWindow,
    calendar: Calendar,
    video: Video,
}

impl App {
    pub fn new(app: &Application, config: &Config) -> Self {
        // let web_context = WebContext::default().unwrap();

        // let webview = WebView::builder()
        //     .web_context(&web_context)
        //     .vexpand(true)
        //     .hexpand(true)
        //     .build();
        // webview.load_uri("https://crates.io/");

        let calendar = Calendar::new(config);
        let video = Video::new(config);

        let paned = Paned::builder()
            .orientation(gtk::Orientation::Horizontal)
            .vexpand(true)
            .hexpand(true)
            .start_child(calendar.widget())
            .end_child(video.widget())
            .css_classes(["paned"])
            .build();

        calendar.widget().set_width_request(300);
        video.widget().set_width_request(600);

        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(900)
            .default_height(600)
            .title("Home Control Panel")
            .child(&paned)
            .visible(true)
            .build();

        App {
            window,
            calendar,
            video,
        }
    }

    pub fn update(&mut self, message: AppMessage) {
        self.calendar.update(message);
    }
}
