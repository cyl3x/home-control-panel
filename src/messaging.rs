use std::sync::OnceLock;

use crate::calendar::CalendarMap;
use crate::enviiq::{self, Overview};
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
    Heating(HeatingMessage),
    Energy(EnergyMessage),
    Switcher(SwitcherMessage),
}

#[derive(Debug)]
pub enum CalendarMessage {
    Fetch,
    Tick,
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
pub enum HeatingMessage {
    Fetch,
    /// Fetch requested by the user, which is throttled
    Refresh,
    Update(std::boxed::Box<Result<Vec<HeatingSystem>, myvaillant::Error>>),
    /// Change a setpoint by steps, sent once the user stops tapping
    Step(Setpoint, i32),
    SendSetpoint(Setpoint),
    Execute(std::boxed::Box<Command>),
    Executed(std::boxed::Box<(Command, Result<(), myvaillant::Error>)>),
    ExpireNotes,
}

impl From<HeatingMessage> for AppMessage {
    fn from(val: HeatingMessage) -> Self {
        Self::Heating(val)
    }
}

#[derive(Debug)]
pub enum EnergyMessage {
    Fetch,
    /// Fetch requested by the user, which is throttled
    Refresh,
    Update(std::boxed::Box<Result<Overview, enviiq::Error>>),
}

impl From<EnergyMessage> for AppMessage {
    fn from(val: EnergyMessage) -> Self {
        Self::Energy(val)
    }
}

/// Pages of the stack that fetch data, their tabs show whether that works
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Page {
    Calendar,
    Heating,
    Energy,
}

#[derive(Debug)]
pub enum SwitcherMessage {
    /// Whether fetching the data of a page succeeded
    Fetched(Page, bool),
    /// Switch to the page, e.g. when its summary on the main screen is tapped
    Show(Page),
}

impl From<SwitcherMessage> for AppMessage {
    fn from(val: SwitcherMessage) -> Self {
        Self::Switcher(val)
    }
}
