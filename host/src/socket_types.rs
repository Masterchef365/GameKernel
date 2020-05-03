use crate::executor::WakeRequest;
use std::sync::mpsc::{channel, Receiver, Sender};

pub type ModuleId = String;
pub type Port = u16;
pub type Handle = u32;

pub struct TwoWayConnection {
    //pub tx: Sender<Box<[u8]>>,
    //pub rx: Receiver<Box<[u8]>>,
    pub tx: Sender<u8>,
    pub rx: Receiver<u8>,
    pub peer: ModuleId,
    pub peer_handle: Handle,
}

impl TwoWayConnection {
    pub fn pair(
        a_peer: ModuleId,
        a_handle: Handle,
        b_peer: ModuleId,
        b_handle: Handle,
    ) -> (Self, Self) {
        let (a_tx, b_rx) = channel();
        let (b_tx, a_rx) = channel();
        (
            TwoWayConnection {
                tx: a_tx,
                rx: a_rx,
                peer: a_peer,
                peer_handle: a_handle,
            },
            TwoWayConnection {
                tx: b_tx,
                rx: b_rx,
                peer: b_peer,
                peer_handle: b_handle,
            },
        )
    }
}
