use crate::app::{AppCommand, AppEvent};
use tokio::sync::mpsc;

pub type CommandSender = mpsc::Sender<AppCommand>;
pub type CommandReceiver = mpsc::Receiver<AppCommand>;
pub type EventSender = mpsc::Sender<AppEvent>;
pub type EventReceiver = mpsc::Receiver<AppEvent>;

pub fn command_channel(buffer: usize) -> (CommandSender, CommandReceiver) {
    mpsc::channel(buffer)
}

pub fn event_channel(buffer: usize) -> (EventSender, EventReceiver) {
    mpsc::channel(buffer)
}

// Buffer sizes per spec: commands 32, events 64
pub const COMMAND_CHANNEL_BUFFER: usize = 32;
pub const EVENT_CHANNEL_BUFFER: usize = 64;
