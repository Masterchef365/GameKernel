use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::stream::{Peekable, StreamExt};

pub type ModuleId = String;
pub type Port = u16;
pub type Handle = u32;

const CHANNEL_CAP: usize = 256;

pub type PeekRecv<T> = Peekable<Receiver<T>>;

// TODO: Replace with a 'ByteChannel' abstraction
pub struct TwoWayConnection {
    pub tx: Sender<u8>,
    pub rx: PeekRecv<u8>,
}

impl TwoWayConnection {
    pub fn pair() -> (Self, Self) {
        let (a_tx, b_rx) = channel(CHANNEL_CAP);
        let (b_tx, a_rx) = channel(CHANNEL_CAP);
        (
            TwoWayConnection { tx: a_tx, rx: a_rx.peekable() },
            TwoWayConnection { tx: b_tx, rx: b_rx.peekable() },
        )
    }
}
