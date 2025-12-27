use std::sync::OnceLock;

use crate::calendar::CalendarMap;

pub type AppSender = async_channel::Sender<AppMessage>;
pub type AppReceiver = async_channel::Receiver<AppMessage>;

static CHANNEL: OnceLock<(AppSender, AppReceiver)> = OnceLock::new();

fn sender() -> &'static AppSender {
    &CHANNEL.get_or_init(|| { async_channel::unbounded() }).0
}

pub fn receiver() -> &'static AppReceiver {
    &CHANNEL.get_or_init(|| { async_channel::unbounded() }).1
}

#[derive(Debug)]
pub enum AppMessage {
    CalendarFetch,
    CalendarMonthPrev,
    CalendarMonthNext,
    CalendarSelectNow,
    CalendarSelectDate(chrono::NaiveDate),
    CalendarSelectIndex(usize),
    CalendarUpdateMap(std::boxed::Box<CalendarMap>),
    CalendarToggleCalendar(uuid::Uuid),
}

pub fn send_message(message: AppMessage) {
    match sender().send_blocking(message) {
        Err(err) => log::error!("Failed to send message: {err}"),
        _ => (),
    };
}
