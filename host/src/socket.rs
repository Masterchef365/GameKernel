use crate::matchmaker::{MatchMakerRequest, MatchMakerRequestBody, MATCHMAKER_MAX_REQ};
use crate::socket_types::*;
use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::sink::{Sink, SinkExt};
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
        let new_handle = self.create_handle();
        let (tx, rx) = channel(MATCHMAKER_MAX_REQ);
        self.connectors.insert(new_handle, rx);
        self.matchmaker
            .try_send(MatchMakerRequest {
                id: addr.into(),
                port,
                body: MatchMakerRequestBody::Connector,
                dest_socket: tx,
            })
            .expect("No matchmaker");
        Poll::Ready(Ok(new_handle))
    }

    /// Create a new listener for a port. Calling this will create a listener that may be passed to
    /// listen()
    pub fn listener_create(&mut self, port: Port) -> Poll<io::Result<Handle>> {
        let new_handle = self.create_handle();
        let (tx, rx) = channel(MATCHMAKER_MAX_REQ);
        self.listeners.insert(new_handle, rx);
        self.matchmaker
            .try_send(MatchMakerRequest {
                id: self.id.clone(),
                port,
                body: MatchMakerRequestBody::Listener,
                dest_socket: tx,
            })
            .expect("No matchmaker");
        Poll::Ready(Ok(new_handle))
    }

    /// Listen for a new connection on this handle.
    pub fn listen(&mut self, handle: Handle, cx: &mut Context) -> Poll<io::Result<Handle>> {
        if let Some(connector) = self.connectors.get_mut(&handle) {
            match connector.poll_next_unpin(cx) {
                Poll::Ready(Some(conn)) => {
                    connector.close();
                    let new_handle = self.create_handle();
                    self.sockets.insert(new_handle, conn);
                    Poll::Ready(Ok(new_handle))
                }
                Poll::Ready(None) => Poll::Ready(Err(io::Error::from(io::ErrorKind::NotFound))),
                Poll::Pending => Poll::Pending,
            }
        } else if let Some(listener) = self.listeners.get_mut(&handle) {
            match listener.poll_next_unpin(cx) {
                Poll::Ready(Some(conn)) => {
                    let new_handle = self.create_handle();
                    self.sockets.insert(new_handle, conn);
                    Poll::Ready(Ok(new_handle))
                }
                Poll::Ready(None) => Poll::Ready(Err(io::Error::from(io::ErrorKind::NotFound))),
                Poll::Pending => Poll::Pending,
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
                match socket.rx.poll_next_unpin(cx) {
                    Poll::Ready(Some(byte)) => {
                        buffer[idx].set(byte);
                        idx += 1;
                        if idx >= buffer.len() {
                            break Poll::Ready(Ok(idx as u32));
                        }
                    }
                    Poll::Ready(None) => {
                        break Poll::Ready(Err(io::Error::from(io::ErrorKind::NotConnected)))
                    }
                    Poll::Pending => {
                        break if idx == 0 {
                            Poll::Pending
                        } else {
                            Poll::Ready(Ok(idx as u32))
                        };
                    }
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
        cx: &mut Context,
    ) -> Poll<io::Result<u32>> {
        if let Some(socket) = self.sockets.get_mut(&handle) {
            use std::pin::Pin;
            let ncerror = |_| io::Error::from(io::ErrorKind::NotConnected);

            let mut n = 0;
            for byte in buffer.iter() {
                let ready = Pin::new(&mut socket.tx)
                    .poll_ready(cx)
                    .map(|v| v.map_err(ncerror))?;
                if ready.is_pending() {
                    break;
                }
                Pin::new(&mut socket.tx)
                    .start_send(byte.get())
                    .map_err(ncerror)?;
                n += 1;
            }

            if n > 0 {
                Poll::Ready(Ok(n))
            } else {
                Poll::Pending
            }
        } else {
            Poll::Ready(Err(io::Error::from(io::ErrorKind::NotFound)))
        }
    }

    /// Return the handles that are supposed to be awake
    pub fn wakes(&mut self) -> Vec<Handle> {
        /* TODO: Implement this somehow, may require changing futures::mpsc::channel's code bit
        let mut wakes: Vec<Handle> = self.listeners
            .iter()
            .chain(self.connectors.iter())
            .filter_map(|handle, listener| {
                if listener.is_ready() {
                    Some(handle)
                } else {
                    None
                }
            }).collect();
        for (handle, socket) in self.sockets.iter() {
            if socket.is_ready() {
                wakes.push(handle);
            }
        }
        */
        self.sockets
            .keys()
            .chain(self.listeners.keys())
            .chain(self.connectors.keys())
            .copied()
            .collect()
    }
}
