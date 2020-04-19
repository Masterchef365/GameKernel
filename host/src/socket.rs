use std::cell::Cell;
use std::collections::{HashMap, VecDeque};
use std::io;
use std::task::Poll;
use crate::maybe::{Handle, Maybe};

pub type ModuleId = String;
pub type Port = u16;

#[derive(Debug, Clone, PartialEq)]
pub struct PeerAddress {
    pub id: ModuleId,
    pub handle: Handle,
}

pub struct Socket {
    pub inbox: VecDeque<u8>,
    pub outbox: VecDeque<u8>,
    pub peer: PeerAddress,
}

impl Socket {
    pub fn new(peer: PeerAddress) -> Self {
        Self {
            peer,
            inbox: Default::default(),
            outbox: Default::default(),
        }
    }
}

#[derive(PartialEq)]
pub enum ListenerType {
    Client(ModuleId, Port),
    Server(Port),
}

pub struct Listener {
    /// Handles to sockets that have been created, but haven't been exposed to the WASM yet
    pub nonconsumed_handles: VecDeque<Handle>,
    /// Actual type of listener and associated data
    pub listener_type: ListenerType,
}

impl Listener {
    /// Create a new listener with the specified type and data
    pub fn new(listener_type: ListenerType) -> Self {
        Self {
            nonconsumed_handles: VecDeque::new(),
            listener_type,
        }
    }
}

#[derive(Default)]
pub struct SocketManager {
    pub listeners: HashMap<Handle, Listener>,
    pub sockets: HashMap<Handle, Socket>,
    pub wakes: Vec<Handle>,
    pub next_handle: Handle,
}

impl SocketManager {
    pub fn new() -> Self {
        Default::default()
    }

    /// Create a new handle, and increment the counter
    fn create_handle(&mut self) -> Handle {
        let handle = self.next_handle;
        self.next_handle += 1;
        handle
    }

    /// Create a new listener and return the handle
    pub fn new_listener(&mut self, listener_type: ListenerType) -> Handle {
        let handle = self.create_handle();
        let listener = Listener::new(listener_type);
        self.listeners.insert(handle, listener);
        handle
    }

    /// Initiate a new connection to a peer. Returns a handle that may be passed to listen().
    pub fn connect(&mut self, addr: &str, port: Port) -> Poll<io::Result<Handle>> {
        Poll::Ready(Ok(
            self.new_listener(ListenerType::Client(addr.into(), port))
        ))
    }

    /// Create a new listener for a port. Calling this will create a listener that may be passed to
    /// listen()
    pub fn listener_create(&mut self, port: Port) -> Poll<io::Result<Handle>> {
        Poll::Ready(Ok(self.new_listener(ListenerType::Server(port))))
    }

    /// Listen for a new connection on this handle.
    pub fn listen(&mut self, handle: Handle) -> Poll<io::Result<Handle>> {
        let mut drop_me = false;
        let ret = if let Some(listener) = self.listeners.get_mut(&handle) {
            match listener.nonconsumed_handles.pop_back() {
                Some(handle) => {
                    if let ListenerType::Client(_, _) = listener.listener_type {
                        drop_me = true;
                    }
                    Poll::Ready(Ok(handle))
                }
                None => Poll::Pending,
            }
        } else {
            todo!("Pass long some error here")
        };
        if drop_me {
            self.listeners.remove(&handle);
        }
        ret
    }

    /// Close this handle
    pub fn close(&mut self, handle: Handle) {
        self.listeners.remove(&handle);
        self.sockets.remove(&handle);
    }

    /// Read from this handle
    pub fn read(&mut self, handle: Handle, buffer: &[Cell<u8>]) -> Poll<io::Result<u32>> {
        if let Some(socket) = self.sockets.get_mut(&handle) {
            let mut idx = 0;
            while let Some(byte) = socket.inbox.pop_front() {
                buffer[idx].set(byte);
                idx += 1;
            }
            match idx {
                0 => Poll::Pending,
                n => Poll::Ready(Ok(n as u32)),
            }
        } else {
            todo!("Pass along some error here")
        }
    }

    /// Write to this handle
    pub fn write(&mut self, handle: Handle, buffer: &[Cell<u8>]) -> Poll<io::Result<u32>> {
        if let Some(socket) = self.sockets.get_mut(&handle) {
            for byte in buffer.iter() {
                socket.outbox.push_back(byte.get());
            }
            Poll::Ready(Ok(buffer.len() as u32))
        } else {
            todo!("Pass along some error here")
        }
    }

    /// Return the handles that are supposed to be awake
    pub fn wakes(&mut self) -> Vec<Handle> {
        std::mem::take(&mut self.wakes)
    }
}
