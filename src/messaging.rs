use std::sync::OnceLock;

use crate::calendar::CalendarMap;

pub type AppSender = async_channel::Sender<AppMessage>;
pub type AppReceiver = async_channel::Receiver<AppMessage>;

static CHANNEL: OnceLock<(AppSender, AppReceiver)> = OnceLock::new();

fn sender() -> &'static AppSender {
    &CHANNEL.get_or_init(|| async_channel::unbounded()).0
}

pub fn receiver() -> &'static AppReceiver {
    &CHANNEL.get_or_init(|| async_channel::unbounded()).1
}

pub fn send_message(message: impl Into<AppMessage>) {
    if let Err(err) = sender().send_blocking(message.into()) {
        log::error!("Failed to send message: {err}")
    };
}

#[derive(Debug)]
pub enum AppMessage {
    Calendar(CalendarMessage),
    Video(VideoMessage),
    Screensaver(ScreensaverMessage),
    Grafana(GrafanaMessage),
}

#[derive(Debug)]
pub enum CalendarMessage {
    Fetch,
    MonthPrev,
    MonthNext,
    SelectNow,
    SelectDate(chrono::NaiveDate),
    SelectGridIndex(usize),
    UpdateMap(std::boxed::Box<CalendarMap>),
    ToggleCalendar(uuid::Uuid),
}

impl From<CalendarMessage> for AppMessage {
    fn from(val: CalendarMessage) -> Self {
        Self::Calendar(val)
    }
}

#[derive(Debug)]
pub enum VideoMessage {
    VideoStateChanged(clapper::PlayerState),
    VideoSelectIndex(usize),
}

impl From<VideoMessage> for AppMessage {
    fn from(val: VideoMessage) -> Self {
        Self::Video(val)
    }
}

#[derive(Debug)]
pub enum ScreensaverMessage {
    Tick,
    Reset,
}

impl From<ScreensaverMessage> for AppMessage {
    fn from(val: ScreensaverMessage) -> Self {
        Self::Screensaver(val)
    }
}

#[derive(Debug)]
pub enum GrafanaMessage {
    Setup,
    RefreshPanels,
}

impl From<GrafanaMessage> for AppMessage {
    fn from(val: GrafanaMessage) -> Self {
        Self::Grafana(val)
    }
}
