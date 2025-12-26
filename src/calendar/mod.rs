pub mod caldav;
mod calendar;
mod event;
mod event_builder;
mod extract;
mod manager;
mod map;

pub use calendar::Calendar;
pub use event::Event;
pub use manager::Manager;
pub use map::CalendarMap;

pub type Color = palette::rgb::Rgb<palette::encoding::Srgb, u8>;
