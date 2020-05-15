use crate::matchmaker::{ConnType, Request, MATCHMAKER_MAX_REQ};
use crate::twoway::*;
use futures::channel::mpsc::{channel, Sender};
use futures::stream::StreamExt;
use protocols::*;
use std::cell::Cell;
use std::collections::HashMap;
use std::io;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

pub struct SocketManager {
    listeners: HashMap<Handle, PeekRecv<TwoWayConnection>>,
    connectors: HashMap<Handle, PeekRecv<TwoWayConnection>>,
    sockets: HashMap<Handle, TwoWayConnection>,
    matchmaker: Sender<Request>,
    next_handle: Handle,
    id: ModuleId,
}

impl SocketManager {
    pub fn new(id: ModuleId, matchmaker: Sender<Request>) -> Self {
        Self {
            id,
            matchmaker,
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
        self.connectors.insert(new_handle, rx.peekable());
        self.matchmaker
            .try_send(Request {
                id: addr.to_string(),
                port,
                conn_type: ConnType::Connector,
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
        self.listeners.insert(new_handle, rx.peekable());
        self.matchmaker
            .try_send(Request {
                id: self.id.clone(),
                port,
                conn_type: ConnType::Listener,
                dest_socket: tx,
            })
            .expect("No matchmaker");
        Poll::Ready(Ok(new_handle))
    }

    /// Listen for a new connection on this handle.
    pub fn listen(&mut self, handle: Handle, cx: &mut Context) -> Poll<io::Result<Handle>> {
        let mut is_connector = false;
        let listeners = &mut self.listeners;

        let listener = self.connectors.get_mut(&handle).or_else(|| {
            is_connector = true;
            listeners.get_mut(&handle)
        });

        if let Some(listener) = listener {
            match listener.poll_next_unpin(cx) {
                Poll::Ready(Some(conn)) => {
                    if is_connector {
                        listener.get_mut().close();
                    }
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
            let mut tmp = vec![0u8; buffer.len()];
            use futures::io::AsyncRead;
            let res = Pin::new(socket).poll_read(cx, &mut tmp);
            if let Poll::Ready(Ok(n)) = res {
                for i in 0..n {
                    buffer[i].set(tmp[i]);
                }
            }
            res.map(|n| n.map(|n| n as u32))
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
            use futures::io::AsyncWrite;
            let tmp: Vec<_> = buffer.iter().map(|v| v.get()).collect();
            Pin::new(socket)
                .poll_write(cx, &tmp)
                .map(|n| n.map(|n| n as u32))
        } else {
            Poll::Ready(Err(io::Error::from(io::ErrorKind::NotFound)))
        }
    }

    /// Return the handles that are supposed to be awake
    pub fn wakes(&mut self, cx: &mut Context) -> Vec<Handle> {
        // Abuse poll_peek() to determine whether there is data behind a socket/listener and wake
        // the appropriate task(s)
        let mut wakes: Vec<Handle> = self
            .listeners
            .iter_mut()
            .chain(self.connectors.iter_mut())
            .filter_map(|(handle, listener)| {
                if Pin::new(&mut *listener).poll_peek(cx).is_ready() {
                    Some(handle)
                } else {
                    None
                }
            })
            .copied()
            .collect();
        for (handle, socket) in self.sockets.iter_mut() {
            if socket.has_data(cx) {
                wakes.push(*handle);
            }
        }
        wakes
    }
}
