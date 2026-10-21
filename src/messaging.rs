use std::sync::OnceLock;

use crate::calendar::CalendarMap;
use crate::myvaillant::{self, Command, HeatingSystem, Setpoint};

pub type AppSender = async_channel::Sender<AppMessage>;
pub type AppReceiver = async_channel::Receiver<AppMessage>;

static CHANNEL: OnceLock<(AppSender, AppReceiver)> = OnceLock::new();

fn sender() -> &'static AppSender {
    &CHANNEL.get_or_init(async_channel::unbounded).0
}

pub fn receiver() -> &'static AppReceiver {
    &CHANNEL.get_or_init(async_channel::unbounded).1
}

pub fn send_message(message: impl Into<AppMessage>) {
    if let Err(err) = sender().send_blocking(message.into()) {
        log::error!("Messaging: Failed to send: {err}");
    }
}

#[derive(Debug)]
pub enum AppMessage {
    Calendar(CalendarMessage),
    Video(VideoMessage),
    Screensaver(ScreensaverMessage),
    Grafana(GrafanaMessage),
    Heating(HeatingMessage),
    Switcher(SwitcherMessage),
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
    CheckVideoState(Option<clapper::PlayerState>),
    VideoSelectIndex(Option<usize>),
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

#[derive(Debug)]
pub enum HeatingMessage {
    Fetch,
    /// Fetch requested by the user, which is throttled
    Refresh,
    Update(std::boxed::Box<Result<Vec<HeatingSystem>, myvaillant::Error>>),
    /// Change a setpoint by steps, sent once the user stops tapping
    Step(Setpoint, i32),
    SendSetpoint(Setpoint),
    Execute(std::boxed::Box<Command>),
    Executed(std::boxed::Box<Result<(), myvaillant::Error>>),
}

impl From<HeatingMessage> for AppMessage {
    fn from(val: HeatingMessage) -> Self {
        Self::Heating(val)
    }
}

/// Pages of the stack that fetch data, their tabs show whether that works
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Page {
    Calendar,
    Heating,
}

#[derive(Debug)]
pub enum SwitcherMessage {
    /// Whether fetching the data of a page succeeded
    Fetched(Page, bool),
}

impl From<SwitcherMessage> for AppMessage {
    fn from(val: SwitcherMessage) -> Self {
        Self::Switcher(val)
    }
}
