use iced::widget::{button, column, container, horizontal_space, text, Row};
use iced::Length;

use super::Message;
use crate::calendar::{Calendar, Manager};
use crate::config;

pub struct CalendarSelection {
    config: Option<config::UuidFilter>,
}

impl CalendarSelection {
    pub const fn new(config: Option<config::UuidFilter>) -> Self {
        Self { config }
    }

    pub fn view<'a>(&'a self, manager: &'a Manager) -> iced::Element<Message> {
        let calendars = manager
            .calendars(self.config.as_ref())
            .map(|(enabled, calendar)| self.view_calendar(*enabled, calendar));

        Row::from_iter(calendars).width(Length::Fill).into()
    }

    fn view_calendar<'a>(
        &'a self,
        enabled: bool,
        calendar: &'a Calendar,
    ) -> iced::Element<Message> {
        let alpha = if enabled { 1.0 } else { 0.5 };

        button(column![
            container(horizontal_space())
                .height(4)
                .style(move |_| style_container(calendar.color.scale_alpha(alpha))),
            container(text(&calendar.name).size(18).wrapping(text::Wrapping::None))
                .center_x(Length::Fill)
                .padding(4),
            container(horizontal_space())
                .height(4)
                .style(move |_| style_container(calendar.color.scale_alpha(alpha))),
        ])
        .padding(0)
        .height(48)
        .style(move |theme, _| style_button(theme, enabled))
        .on_press(Message::ToggleCalendar(calendar.uid))
        .into()
    }
}

fn style_button(theme: &iced::Theme, enabled: bool) -> button::Style {
    let palette = theme.extended_palette();

    button::Style {
        background: None,
        border: iced::border::width(0),
        text_color: if enabled {
            palette.secondary.base.text
        } else {
            palette.secondary.strong.color
        },
        ..Default::default()
    }
}

fn style_container(color: iced::Color) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(color)),
        ..Default::default()
    }
}
