use chrono::NaiveDateTime;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{column, row, scrollable, text};
use iced::{Alignment, Length};
use iced_font_awesome::fa_icon_solid;

use crate::calendar::{self, Calendar, Manager};
use crate::config;

use super::{Dates, Message};

pub struct Event {
    config: Option<config::UuidFilter>,
}

impl Event {
    pub const fn new(config: Option<config::UuidFilter>) -> Self {
        Self { config }
    }

    pub fn view<'a>(&'a self, manager: &'a Manager, dates: &Dates) -> iced::Element<'a, Message> {
        let event = manager
            .events_between(dates.now.date(), dates.now.date(), self.config.as_ref())
            .next()
            .map(|item| self.view_event(item));

        let entry = column![].push(event);

        scrollable(entry)
            .width(Length::Fill)
            .direction(Direction::Horizontal(Scrollbar::new()))
            .into()
    }

    fn view_event<'a>(
        &'a self,
        (calendar, _, event): (&'a Calendar, &NaiveDateTime, &'a calendar::Event),
    ) -> iced::Element<'a, Message> {
        row![
            fa_icon_solid("circle").color(calendar.color).size(26.0),
            text(event.summary.clone()).size(29),
        ]
        .height(36)
        .align_y(Alignment::Center)
        .spacing(8)
        .into()
    }
}
