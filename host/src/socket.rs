use crate::executor::{WakeRequest, WakeRequestBody};
use crate::matchmaker::{MatchConnector, MatchListener, MatchRequest};
use crate::socket_types::*;
use std::cell::Cell;
use std::collections::{HashMap, VecDeque};
use std::io;
use std::sync::mpsc::Sender;
use std::task::Poll;

struct Listener {
    is_connector: bool,
    new_connections: VecDeque<TwoWayConnection>,
}

impl Listener {
    pub fn new(is_connector: bool) -> Self {
        Self {
            is_connector,
            new_connections: VecDeque::new(),
        }
    }
}

pub struct SocketManager {
    listeners: HashMap<Handle, Listener>,
    sockets: HashMap<Handle, TwoWayConnection>,
    wakes: Vec<Handle>,
    next_handle: Handle,
    wake_sender: Sender<WakeRequest>,
    match_sender: Sender<MatchRequest>,
    id: ModuleId,
}

impl SocketManager {
    pub fn wake(&mut self, wake: WakeRequestBody) {
        match wake {
            WakeRequestBody::EndPointConnected(handle, connection) => {
                if let Some(listener) = self.listeners.get_mut(&handle) {
                    listener.new_connections.push_front(connection);
                    self.wakes.push(handle);
                }
            }
            WakeRequestBody::Data(handle) => self.wakes.push(handle),
        }
    }

    pub fn new(
        wake_sender: Sender<WakeRequest>,
        match_sender: Sender<MatchRequest>,
        id: ModuleId,
    ) -> Self {
        Self {
            id,
            wake_sender,
            match_sender,
            wakes: Vec::new(),
            next_handle: 0,
            sockets: HashMap::new(),
            listeners: HashMap::new(),
        }
    }

    /// Create a new handle, and increment the counter
    fn create_handle(&mut self) -> Handle {
        let handle = self.next_handle;
        self.next_handle += 1;
        handle
    }

    /// Initiate a new connection to a peer. Returns a handle that may be passed to listen().
    pub fn connect(&mut self, addr: &str, port: Port) -> Poll<io::Result<Handle>> {
        let handle = self.create_handle();
        self.listeners.insert(handle, Listener::new(true));
        let _ = self
            .match_sender
            .send(MatchRequest::Connector(MatchConnector {
                dest_module: addr.into(),
                dest_port: port,
                handle,
                module: self.id.clone(),
            }));
        Poll::Ready(Ok(handle))
    }

    /// Create a new listener for a port. Calling this will create a listener that may be passed to
    /// listen()
    pub fn listener_create(&mut self, port: Port) -> Poll<io::Result<Handle>> {
        let handle = self.create_handle();
        self.listeners.insert(handle, Listener::new(false));
        let _ = self
            .match_sender
            .send(MatchRequest::Listener(MatchListener {
                port,
                handle,
                module: self.id.clone(),
            }));
        Poll::Ready(Ok(handle))
    }

    /// Listen for a new connection on this handle.
    pub fn listen(&mut self, handle: Handle) -> Poll<io::Result<Handle>> {
        let mut drop_me = false;
        let ret = if let Some(listener) = self.listeners.get_mut(&handle) {
            match listener.new_connections.pop_back() {
                Some(connection) => {
                    drop_me = listener.is_connector;
                    let handle = self.create_handle();
                    self.sockets.insert(handle, connection);
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
            while let Ok(byte) = socket.rx.try_recv() {
                buffer[idx].set(byte);
                idx += 1;
                if idx >= buffer.len() {
                    break;
                }
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
            self.wake_sender
                .send(WakeRequest {
                    module: socket.peer.clone(),
                    body: WakeRequestBody::Data(socket.peer_handle),
                })
                .unwrap();
            for byte in buffer.iter() {
                socket.tx.send(byte.get()).unwrap();
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
