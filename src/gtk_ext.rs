use gtk::prelude::{IsA, WidgetExt};

/// Trait that extends [`gtk::prelude::WidgetExt`].
///
/// This trait's main goal is to reduce redundant code and
/// to provide helpful methods for the widgets macro of relm4-macros.
pub trait AppWidgetExt {
    /// Set margin at start, end, top and bottom all at once.
    fn set_margin_all(&self, margin: i32) {
        self.set_margin_horizontal(margin);
        self.set_margin_vertical(margin);
    }

    /// Set margin at top and bottom at once.
    fn set_margin_vertical(&self, margin: i32);

    /// Set margin at start and end at once.
    fn set_margin_horizontal(&self, margin: i32);

    /// Set both horizontal and vertical expand properties at once.
    fn set_expand(&self, expand: bool);

    /// Set both horizontal and vertical align properties at once.
    fn set_align(&self, align: gtk::Align);

    /// Add class name if active is [`true`] and
    /// remove class name if active is [`false`]
    fn set_class_active(&self, class: &str, active: bool);

    /// Add inline CSS instructions to a widget.
    /// ```
    /// # gtk::init().unwrap();
    /// # let widget = gtk::Button::new();
    /// widget.inline_css("border: 1px solid red");
    /// ```
    fn inline_css(&self, style: &str);
}

impl<T: IsA<gtk::Widget>> AppWidgetExt for T {
    fn set_margin_vertical(&self, margin: i32) {
        self.set_margin_top(margin);
        self.set_margin_bottom(margin);
    }

    fn set_margin_horizontal(&self, margin: i32) {
        self.set_margin_start(margin);
        self.set_margin_end(margin);
    }

    fn set_class_active(&self, class: &str, active: bool) {
        if active {
            self.add_css_class(class);
        } else {
            self.remove_css_class(class);
        }
    }

    fn set_expand(&self, expand: bool) {
        self.set_hexpand(expand);
        self.set_vexpand(expand);
    }

    fn set_align(&self, align: gtk::Align) {
        self.set_halign(align);
        self.set_valign(align);
    }

    #[allow(deprecated)]
    fn inline_css(&self, style: &str) {
        use gtk::prelude::StyleContextExt;

        let context = self.style_context();
        let provider = gtk::CssProvider::new();

        let data = if style.ends_with(';') {
            ["*{", style, "}"].concat()
        } else {
            ["*{", style, ";}"].concat()
        };

        provider.load_from_data(&data);
        context.add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_USER + 1);
    }
}
