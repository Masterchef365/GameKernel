use futures::channel::mpsc::{channel, Receiver, Sender};

pub type ModuleId = String;
pub type Port = u16;
pub type Handle = u32;

const CHANNEL_CAP: usize = 256;

pub struct TwoWayConnection {
    pub tx: Sender<u8>,
    pub rx: Receiver<u8>,
}

impl TwoWayConnection {
    pub fn pair() -> (Self, Self) {
        let (a_tx, b_rx) = channel(CHANNEL_CAP);
        let (b_tx, a_rx) = channel(CHANNEL_CAP);
        (
            TwoWayConnection { tx: a_tx, rx: a_rx },
            TwoWayConnection { tx: b_tx, rx: b_rx },
        )
    }
}
