use chrono::NaiveDateTime;
use chrono_tz::Europe;
use iced::theme::palette;
use iced::widget::{column, container, row, scrollable, text, vertical_space, Column};
use iced::{Alignment, Length};

use crate::calendar::{Calendar, Event, Manager};
use crate::config;

use super::{Dates, Message};

pub struct Day {
    config: Option<config::UuidFilter>,
}

impl Day {
    pub const fn new(config: Option<config::UuidFilter>) -> Self {
        Self { config }
    }

    pub fn view<'a>(&'a self, manager: &'a Manager, dates: &Dates) -> iced::Element<Message> {
        let events = manager
            .events_between(dates.selected, dates.selected, self.config.as_ref())
            .map(|item| self.view_event(item));

        let entries = Column::from_iter(events).spacing(16);

        scrollable(entries)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_event<'a>(
        &'a self,
        (calendar, start, event): (&'a Calendar, &NaiveDateTime, &'a Event),
    ) -> iced::Element<Message> {
        row![
            container(vertical_space())
                .width(8)
                .style(|_| style_event_indicator(calendar.color)),
            column![
                text(event.summary.clone()).size(20),
                text(event.description.as_deref().unwrap_or(""))
                    .style(style_event_description)
                    .size(20),
            ]
            .spacing(2),
            column![text(
                start
                    .and_utc()
                    .with_timezone(&Europe::Berlin)
                    .format("%H:%M")
                    .to_string()
            )
            .wrapping(text::Wrapping::None)
            .size(16)]
            .width(Length::Fill)
            .align_x(Alignment::End),
        ]
        .height(54)
        .align_y(Alignment::Center)
        .spacing(8)
        .into()
    }
}

fn style_event_indicator(color: iced::Color) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(color)),
        border: iced::border::rounded(12),
        ..Default::default()
    }
}


fn style_event_description(theme: &iced::Theme) -> text::Style {
    let palette = theme.palette();

    text::Style {
        color: Some(palette.text.scale_alpha(0.8)),
    }
}
