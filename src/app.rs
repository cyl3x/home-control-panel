use gtk::Application;

pub struct App {
}

impl App {
    pub fn new(app: &Application) -> Self {
        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(800)
            .default_height(600)
            .title("Home Control Panel")
            .build();

        window.present();

        App {

        }
    }
}
