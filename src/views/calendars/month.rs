use std::collections::BTreeMap;

use chrono::{Datelike, Duration, Locale, NaiveDate};
use iced::widget::{button, column, container, row, text, Row};
use iced::{Border, Font, Length};
use iced_font_awesome::fa_icon_solid;
use uuid::Uuid;

use super::{Dates, Message};
use crate::calendar::Manager;
use crate::config;

const DURATION: chrono::TimeDelta = Duration::days(41);

pub struct Month {
    config: Option<config::UuidFilter>,
}

impl Month {
    pub const fn new(config: Option<config::UuidFilter>) -> Self {
        Self { config }
    }

    pub fn view(&self, manager: &Manager, dates: &Dates) -> iced::Element<Message> {
        let start = start_grid_date(dates.selected);

        let controls = self.view_controls(dates.selected);

        let indicators = manager
            .calendars_between(start, start + DURATION, self.config.as_ref())
            .fold(
                BTreeMap::new(),
                |mut map: BTreeMap<NaiveDate, BTreeMap<Uuid, iced::Color>>,
                 (event_start, calendar)| {
                    map.entry(event_start.date())
                        .or_default()
                        .entry(calendar.uid)
                        .or_insert(calendar.color);

                    map
                },
            );

        let days = row![
            self.view_day("Mo"),
            self.view_day("Di"),
            self.view_day("Mi"),
            self.view_day("Do"),
            self.view_day("Fr"),
            self.view_day("Sa"),
            self.view_day("So"),
        ];

        let month_grid = column![days].extend((0..6).map(|row| {
            Row::from_iter((0..7).map(|col| {
                let date = start + Duration::days((row * 7 + col) as i64);
                self.view_button(dates, date, indicators.get(&date))
            }))
            .into()
        }));

        column![container(controls), container(month_grid),]
            .spacing(16)
            .into()
    }

    fn view_controls(&self, selected: NaiveDate) -> iced::Element<Message> {
        row![
            button(fa_icon_solid("caret-left").size(42.0))
                .style(style_month_button)
                .on_press_with(move || Message::SelectDate(selected - chrono::Months::new(1))),
            text(
                selected
                    .format_localized("%B %Y", Locale::de_DE)
                    .to_string()
            )
            .center()
            .width(iced::Length::Fill)
            .size(24.0)
            .font(Font {
                family: iced::font::Family::Name("Inter"),
                weight: iced::font::Weight::Bold,
                ..Font::default()
            })
            .wrapping(text::Wrapping::None),
            button(fa_icon_solid("caret-right").size(42.0))
                .style(style_month_button)
                .on_press_with(move || Message::SelectDate(selected + chrono::Months::new(1))),
        ]
        .height(50)
        .align_y(iced::Alignment::Center)
        .into()
    }

    fn view_button(
        &self,
        dates: &Dates,
        date: NaiveDate,
        indicators: Option<&BTreeMap<Uuid, iced::Color>>,
    ) -> iced::Element<Message> {
        let is_month = dates.is_month(date);

        let content = container(
            column![
                text(date.day()).style(match is_month {
                    true => style_text_on_month,
                    false => style_text_off_month,
                }),
                self.view_event_indicators(is_month, indicators),
            ]
            .align_x(iced::Alignment::Center),
        )
        .align_x(iced::Alignment::Center)
        .align_y(iced::Alignment::Center);

        button(content)
            .width(iced::Length::Fill)
            .height(48)
            .padding(4)
            .style(match (dates.is_today(date), dates.is_selected(date)) {
                (_, true) => style_selected,
                (true, _) => style_today,
                _ => style_normal,
            })
            .on_press(Message::SelectDate(date))
            .into()
    }

    fn view_event_indicators(
        &self,
        is_month: bool,
        indicators: Option<&BTreeMap<Uuid, iced::Color>>,
    ) -> iced::Element<Message> {
        let dots = match indicators {
            Some(indicators) => indicators.values().map(|color| {
                fa_icon_solid("circle")
                    .color(match is_month {
                        true => *color,
                        false => color.scale_alpha(0.5),
                    })
                    .size(12.0)
                    .into()
            }),
            None => return Row::new().into(),
        };

        Row::from_iter(dots).spacing(4).into()
    }

    fn view_day<'a>(&self, date: &'a str) -> iced::Element<'a, Message> {
        text(date)
            .size(18.0)
            .font(Font {
                family: iced::font::Family::Name("Inter"),
                weight: iced::font::Weight::Semibold,
                ..Font::default()
            })
            .center()
            .width(Length::Fill)
            .into()
    }
}

fn start_grid_date(date: NaiveDate) -> NaiveDate {
    let mut first = date.with_day(1).unwrap();

    while first.weekday() != chrono::Weekday::Mon {
        first = first.pred_opt().unwrap_or(first);
    }

    first
}

pub fn style_month_button(theme: &iced::Theme, _: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    button::Style {
        text_color: palette.primary.strong.text,
        background: Some(palette.primary.strong.color.into()),
        border: Border::default().rounded(3),
        ..Default::default()
    }
}

pub fn style_text_on_month(theme: &iced::Theme) -> text::Style {
    let palette = theme.extended_palette();

    text::Style {
        color: palette.secondary.base.text.into(),
    }
}

pub fn style_text_off_month(theme: &iced::Theme) -> text::Style {
    let palette = theme.extended_palette();

    text::Style {
        color: palette.secondary.strong.color.into(),
    }
}

pub fn style_normal(theme: &iced::Theme, _: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    button::Style {
        background: None,
        border: iced::Border {
            width: 1.0,
            color: palette.background.strong.color,
            ..iced::Border::default()
        },
        ..Default::default()
    }
}

pub fn style_selected(theme: &iced::Theme, _: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    button::Style {
        background: Some(palette.primary.strong.color.into()),
        border: iced::Border {
            width: 1.0,
            color: palette.background.strong.color,
            ..iced::Border::default()
        },
        ..Default::default()
    }
}

pub fn style_today(theme: &iced::Theme, _: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    button::Style {
        background: Some(palette.primary.weak.color.into()),
        border: iced::Border {
            width: 1.0,
            color: palette.background.strong.color,
            ..iced::Border::default()
        },
        ..Default::default()
    }
}
