use gtk::glib;
use webkit6::{Settings, WebContext, WebView};

use crate::{config, messaging, prelude::*};

pub struct GrafanaWidget {
    wrapper: gtk::Overlay,
    webviews: Vec<(url::Url, WebView)>,
    login_view: Option<WebView>,
    panels_view: gtk::Box,
    spinner: gtk::Spinner,
}

impl GrafanaWidget {
    pub fn new(config: &config::Config) -> Self {
        let grid = gtk::Grid::new();
        grid.set_row_homogeneous(false);
        grid.set_column_homogeneous(false);
        grid.set_row_spacing(12);
        grid.set_column_spacing(12);
        grid.add_css_class("grafana__panels");

        let refresh_button = gtk::Button::from_icon_name("view-refresh-symbolic");
        refresh_button.set_valign(gtk::Align::Center);
        refresh_button.set_halign(gtk::Align::End);
        refresh_button.connect_clicked(|_| {
            messaging::send_message(messaging::GrafanaMessage::RefreshPanels);
        });

        let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        toolbar.set_hexpand(true);
        toolbar.set_halign(gtk::Align::End);
        toolbar.add_css_class("grafana__toolbar");
        toolbar.append(&refresh_button);

        let panels_view = gtk::Box::new(gtk::Orientation::Vertical, 0);
        panels_view.set_expand(true);
        panels_view.append(&toolbar);
        panels_view.append(&grid);

        let spinner = gtk::Spinner::new();
        spinner.set_expand(true);
        spinner.set_halign(gtk::Align::Center);
        spinner.set_valign(gtk::Align::Center);
        spinner.add_css_class("large");
        spinner.start();

        let wrapper = gtk::Overlay::new();
        wrapper.add_css_class("grafana");
        wrapper.add_css_class("spinner-active");
        wrapper.set_expand(true);
        wrapper.set_child(Some(&panels_view));
        wrapper.add_overlay(&spinner);

        spinner.connect_visible_notify(glib::clone!(#[strong] wrapper, move |spinner| {
            if spinner.is_visible() {
                wrapper.add_css_class("spinner-active");
                spinner.start();
            } else {
                wrapper.remove_css_class("spinner-active");
                spinner.stop();
            }
        }));

        let web_context = WebContext::new();
        web_context.set_cache_model(webkit6::CacheModel::WebBrowser);

        let base_cache_dir = std::env::home_dir().map(|p| p.join(".cache/home-control-panel"));
        let data_dir = base_cache_dir.as_ref().map(|p| p.join("webview-data").to_string_lossy().into_owned());
        let cache_dir = base_cache_dir.as_ref().map(|p| p.join("webview-cache").to_string_lossy().into_owned());
        let cookie_dir = base_cache_dir.map(|p| p.join("webview-cookies/cookies.sqlite").to_string_lossy().into_owned());

        let network_session = webkit6::NetworkSession::new(data_dir.as_deref(), cache_dir.as_deref());

        if let Some(cookie_dir) = &cookie_dir {
            std::fs::create_dir_all(std::path::Path::new(cookie_dir).parent().expect("Should have parent")).ok();
            network_session.cookie_manager().unwrap().set_persistent_storage(cookie_dir, webkit6::CookiePersistentStorage::Sqlite);
        }

        let (js, login_view) = config.grafana.login.as_ref().map_or_else(|| (None, None), |login| {
            log::info!("Grafana: logging in");

            let js = format!(
                r#"
                const username = "{}";
                const password = "{}";
                {}
                "#,
                login.username,
                login.password,
                login.js_script
            );

            let uri = login.url.as_str().to_string();

            // @todo on init
            network_session.cookie_manager().unwrap().connect_changed(move |cookie_manager| {
                glib::spawn_future_local(glib::clone!(#[strong] uri, #[strong] cookie_manager, async move {
                    let has_grafana_session = cookie_manager
                        .cookies_future(&uri)
                        .await
                        .unwrap_or_default()
                        .iter_mut()
                        .any(|cookie| matches!(cookie.name(), Some(name) if name == "grafana_session"));

                    if has_grafana_session {
                        messaging::send_message(messaging::GrafanaMessage::Setup);
                    }
                }));
            });

            let settings = Settings::new();
            settings.set_enable_javascript(true);
            settings.set_enable_developer_extras(login.developer_extras);

            let webview = WebView::builder()
                .web_context(&web_context)
                .network_session(&network_session)
                .settings(&settings)
                .hexpand(true)
                .vexpand(true)
                .css_classes(["grafana__login__webview"])
                .build();

            webview.connect_load_changed(glib::clone!(#[strong] webview, #[strong] js, move |_, event| {
                if event == webkit6::LoadEvent::Finished && let Some(uri) = webview.uri() && uri.starts_with("http") {
                    glib::spawn_future_local(glib::clone!(#[strong] webview, #[strong] js, async move {
                        let _ = webview.evaluate_javascript_future(&js, None, None).await;
                    }));
                }
            }));

            let uri = login.url.as_str().to_string();
            glib::timeout_add_seconds_local_once(1, glib::clone!(#[strong] webview, move || {
                webview.load_uri(&uri);
                webview.inspector().unwrap().show();
            }));

            wrapper.set_child(Some(&webview));

            (Some(js), Some(webview))
        });

        let mut webviews = Vec::new();
        for panel in &config.grafana.panels {
            let settings = Settings::new();
            settings.set_enable_javascript(true);
            settings.set_enable_developer_extras(panel.developer_extras);

            let webview = WebView::builder()
                .web_context(&web_context)
                .network_session(&network_session)
                .settings(&settings)
                .hexpand(true)
                .vexpand(true)
                .build();

            if let Some(js) = &js {
                log::info!("Grafana: setting up JS injection");
                webview.connect_load_changed(glib::clone!(#[strong] webview, #[strong] js, move |_, event| {
                    if event == webkit6::LoadEvent::Finished && let Some(uri) = webview.uri() && uri.starts_with("http") {
                        glib::spawn_future_local(glib::clone!(#[strong] webview, #[strong] js, async move {
                            let _ = webview.evaluate_javascript_future(&js, None, None).await;
                        }));
                    }
                }));
            }

            let webview_wrapper = gtk::Box::new(gtk::Orientation::Vertical, 0);
            webview_wrapper.set_expand(true);
            webview_wrapper.add_css_class("grafana__panel__webview");
            webview_wrapper.append(&webview);
            webview_wrapper.set_overflow(gtk::Overflow::Hidden);

            grid.attach(&webview_wrapper, panel.column.into(), panel.row.into(), panel.width.into(), panel.height.into());

            webviews.push((panel.url.clone(), webview));
        }

        Self {
            wrapper,
            webviews,
            login_view,
            panels_view,
            spinner,
        }
    }

    pub const fn widget(&self) -> &gtk::Overlay {
        &self.wrapper
    }

    pub fn update(&mut self, message: messaging::GrafanaMessage) {
        match message {
            messaging::GrafanaMessage::Setup => {
                if self.login_view.is_none() {
                    return;
                }

                log::info!("Grafana: login successful, setup panels");

                if let Some(login_view) = self.login_view.take() {
                    login_view.stop_loading();
                    login_view.unparent();
                }

                self.wrapper.set_child(Some(&self.panels_view));

                for (panel, webview) in &self.webviews {
                    // webview.inspector().unwrap().show();
                    webview.set_visible(true);

                    glib::timeout_add_seconds_local_once(1, glib::clone!(#[strong] webview, #[strong] panel, move || {
                        webview.load_uri(panel.as_str());
                    }));
                }

                self.spinner.set_visible(false);
            }
            messaging::GrafanaMessage::RefreshPanels => {
                for (panel, webview) in &self.webviews {
                    log::info!("Grafana: refreshing panel");

                    webview.stop_loading();

                    glib::timeout_add_seconds_local_once(1, glib::clone!(#[strong] webview, #[strong] panel, move || {
                        webview.load_uri(panel.as_str());
                    }));
                }
            }
        }
    }
}
