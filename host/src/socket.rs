use crate::matchmaker::{MatchMakerRequest, MatchMakerRequestBody, MATCHMAKER_MAX_REQ};
use crate::socket_types::*;
use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use std::cell::Cell;
use std::collections::{HashMap, VecDeque};
use std::io;
use std::task::Context;
use std::task::Poll;

pub struct SocketManager {
    listeners: HashMap<Handle, Receiver<TwoWayConnection>>,
    connectors: HashMap<Handle, Receiver<TwoWayConnection>>,
    sockets: HashMap<Handle, TwoWayConnection>,
    matchmaker: Sender<MatchMakerRequest>,
    wakes: Vec<Handle>,
    next_handle: Handle,
    id: ModuleId,
}

impl SocketManager {
    pub fn new(id: ModuleId, matchmaker: Sender<MatchMakerRequest>) -> Self {
        Self {
            id,
            matchmaker,
            wakes: Vec::new(),
            next_handle: 0,
            sockets: HashMap::new(),
            listeners: HashMap::new(),
            connectors: HashMap::new(),
        }
    }

    /// Create a new handle, and increment the counter
    fn create_handle(&mut self) -> Handle {
        let handle = self.next_handle;
        self.next_handle += 1;
        handle
    }
}

impl SocketManager {
    /// Initiate a new connection to a peer. Returns a handle that may be passed to listen().
    pub fn connect(&mut self, addr: &str, port: Port) -> Poll<io::Result<Handle>> {
        let handle = self.create_handle();
        let (tx, rx) = channel(MATCHMAKER_MAX_REQ);
        self.connectors.insert(handle, rx);
        self.matchmaker
            .try_send(MatchMakerRequest {
                id: addr.into(),
                port,
                body: MatchMakerRequestBody::Connector,
                dest_socket: tx,
            })
            .expect("No matchmaker");
        Poll::Ready(Ok(handle))
    }

    /// Create a new listener for a port. Calling this will create a listener that may be passed to
    /// listen()
    pub fn listener_create(&mut self, port: Port) -> Poll<io::Result<Handle>> {
        let handle = self.create_handle();
        let (tx, rx) = channel(MATCHMAKER_MAX_REQ);
        self.listeners.insert(handle, rx);
        self.matchmaker
            .try_send(MatchMakerRequest {
                id: self.id.clone(),
                port,
                body: MatchMakerRequestBody::Listener,
                dest_socket: tx,
            })
            .expect("No matchmaker");
        Poll::Ready(Ok(handle))
    }

    /// Listen for a new connection on this handle.
    pub fn listen(&mut self, handle: Handle, cx: &mut Context) -> Poll<io::Result<Handle>> {
        if let Some(connector) = self.connectors.get_mut(&handle) {
            if let Some(conn) = connector.try_next().unwrap() {
                connector.close();
                let new_handle = self.create_handle();
                self.sockets.insert(new_handle, conn);
                Poll::Ready(Ok(handle))
            } else {
                Poll::Pending
            }
        } else if let Some(listener) = self.listeners.get_mut(&handle) {
            if let Some(conn) = listener.try_next().unwrap() {
                let new_handle = self.create_handle();
                self.sockets.insert(new_handle, conn);
                Poll::Ready(Ok(handle))
            } else {
                Poll::Pending
            }
        } else {
            Poll::Ready(Err(io::Error::from(io::ErrorKind::NotFound)))
        }
    }

    /// Close this handle
    pub fn close(&mut self, handle: Handle) {
        self.listeners.remove(&handle);
        self.connectors.remove(&handle);
        self.sockets.remove(&handle);
    }

    /// Read from this handle
    pub fn read(
        &mut self,
        handle: Handle,
        buffer: &[Cell<u8>],
        cx: &mut Context,
    ) -> Poll<io::Result<u32>> {
        if let Some(socket) = self.sockets.get_mut(&handle) {
            let mut idx = 0;
            loop {
                let ret = socket.rx.poll_next_unpin(cx);
                match ret {
                    Poll::Ready(Some(byte)) => {
                        buffer[idx].set(byte);
                        idx += 1;
                        cx.waker().wake_by_ref();
                    }
                    Poll::Ready(None) => break Poll::Ready(Ok(idx as u32)),
                    Poll::Pending => break Poll::Pending,
                }
            }
        } else {
            Poll::Ready(Err(io::Error::from(io::ErrorKind::NotFound)))
        }
    }

    /// Write to this handle
    pub fn write(
        &mut self,
        handle: Handle,
        buffer: &[Cell<u8>],
    ) -> Poll<io::Result<u32>> {
        if let Some(socket) = self.sockets.get_mut(&handle) {
            for byte in buffer.iter() {
                socket.tx.try_send(byte.get()).unwrap();
            }
            Poll::Ready(Ok(buffer.len() as u32))
        } else {
            Poll::Ready(Err(io::Error::from(io::ErrorKind::NotFound)))
        }
    }

    /// Return the handles that are supposed to be awake
    pub fn wakes(&mut self) -> Vec<Handle> {
        // TODO: Actually implement that
        //std::mem::take(&mut self.wakes)
        self.sockets
            .keys()
            .chain(self.listeners.keys())
            .chain(self.connectors.keys())
            .copied()
            .collect()
    }
}
