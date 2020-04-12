mod module;
use module::{Module, WasmSocket};
use std::cell::Cell;
use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::io;
use std::task::Poll;

use libplugin::Handle;

type ModuleId = String;
type Packet = Box<[u8]>;
type Port = u16;

#[derive(Debug, Clone, PartialEq)]
struct PeerAddress {
    id: ModuleId,
    handle: Handle,
}

struct Socket {
    inbox: VecDeque<u8>,
    outbox: VecDeque<u8>,
    peer: PeerAddress,
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
enum ListenerType {
    Client(ModuleId, Port),
    Server(Port),
}

struct Listener {
    /// Handles to sockets that have been created, but haven't been exposed to the WASM yet
    nonconsumed_handles: VecDeque<Handle>,
    /// Actual type of listener and associated data
    listener_type: ListenerType,
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
struct SocketManager {
    pub listeners: HashMap<Handle, Listener>,
    pub sockets: HashMap<Handle, Socket>,
    pub wakes: Vec<Handle>,
    pub next_handle: Handle,
}

impl SocketManager {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    /// Create a new handle, and increment the counter
    fn create_handle(&mut self) -> Handle {
        let handle = self.next_handle;
        self.next_handle += 1;
        handle
    }

    /// Create a new listener and return the handle
    fn new_listener(&mut self, listener_type: ListenerType) -> Handle {
        let handle = self.create_handle();
        let listener = Listener::new(listener_type);
        self.listeners.insert(handle, listener);
        handle
    }

    /// Create a new socket connected to the specified peer address
    pub fn new_socket(&mut self, peer: PeerAddress) -> Handle {
        let handle = self.create_handle();
        self.sockets.insert(handle, Socket::new(peer));
        handle
    }
}

impl WasmSocket for SocketManager {
    fn connect(&mut self, addr: &str, port: Port) -> Poll<io::Result<Handle>> {
        println!("Connect {}:{}", addr, port);
        Poll::Ready(Ok(
            self.new_listener(ListenerType::Client(addr.into(), port))
        ))
    }

    fn listener_create(&mut self, port: Port) -> Poll<io::Result<Handle>> {
        println!("Listner create {}", port);
        Poll::Ready(Ok(self.new_listener(ListenerType::Server(port))))
    }

    fn listen(&mut self, handle: Handle) -> Poll<io::Result<Handle>> {
        println!("Listen {}", handle);
        if let Some(listener) = self.listeners.get_mut(&handle) {
            match listener.nonconsumed_handles.pop_back() {
                Some(handle) => Poll::Ready(Ok(handle)),
                None => Poll::Pending,
            }
        } else {
            Poll::Ready(Err(todo!("Error out here")))
        }
    }

    fn close(&mut self, handle: Handle) {
        println!("Close {}", handle);
        self.listeners.remove(&handle);
        self.sockets.remove(&handle);
    }

    fn read(&mut self, handle: Handle, _buffer: &[Cell<u8>]) -> Poll<io::Result<u32>> {
        println!("Read {}", handle);
        Poll::Ready(Ok(0))
    }

    fn write(&mut self, handle: Handle, _buffer: &[Cell<u8>]) -> Poll<io::Result<u32>> {
        println!("Write {}", handle);
        Poll::Ready(Ok(0))
    }

    fn wakes(&mut self) -> Vec<Handle> {
        std::mem::take(&mut self.wakes)
    }
}

struct ManagedInstance {
    pub module: Module<SocketManager>,
    pub socketman: SocketManager,
}

struct Manager {
    instances: HashMap<ModuleId, ManagedInstance>,
}

/// Note: Any items inserted during iteration will not be iterated over.
fn mutable_iterate<K: std::hash::Hash + Eq + Clone, V>(
    map: &mut HashMap<K, V>,
    f: impl Fn((&K, &mut V), &mut HashMap<K, V>),
) {
    let keys: Vec<K> = map.keys().cloned().collect();
    for key in keys {
        if let Some(mut value) = map.remove(&key) {
            f((&key, &mut value), map);
            map.insert(key, value);
        }
    }
}

impl Manager {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
        }
    }

    pub fn add_module(&mut self, name: impl Into<ModuleId>, module: Module<SocketManager>) {
        self.instances.insert(
            name.into(),
            ManagedInstance {
                module,
                socketman: SocketManager::new(),
            },
        );
    }

    pub fn run(&mut self) {
        mutable_iterate(&mut self.instances, Self::run_module);
    }

    fn run_module(
        (us_id, us_module): (&ModuleId, &mut ManagedInstance),
        others: &mut HashMap<ModuleId, ManagedInstance>,
    ) {
        for socket in &mut us_module.socketman.sockets.values_mut() {
            if let Some(peer) = others.get_mut(&socket.peer.id) {
                if let Some(peer_socket) = peer.socketman.sockets.get_mut(&socket.peer.handle) {
                    if !socket.outbox.is_empty() {
                        peer_socket.inbox.extend(socket.outbox.drain(..));
                        peer.socketman.wakes.push(socket.peer.handle);
                    }
                }
            }
        }

        let us_listeners = &mut us_module.socketman.listeners;
        let us_next_handle = &mut us_module.socketman.next_handle;
        let us_sockets = &mut us_module.socketman.sockets;

        for (us_handle, us_listener) in us_listeners {
            if let ListenerType::Client(peer, port) = &us_listener.listener_type {
                if let Some(peer_module) = others.get_mut(peer) {
                    let peer_listeners = &mut peer_module.socketman.listeners;
                    let peer_next_handle = &mut peer_module.socketman.next_handle;
                    let peer_sockets = &mut peer_module.socketman.sockets;

                    for (peers_handle, peers_listener) in peer_listeners {
                        if peers_listener.listener_type == ListenerType::Server(*port) {
                            // Connect them to us
                            let peer_new_handle = *peer_next_handle;
                            *peer_next_handle += 1;
                            peer_sockets.insert(
                                peer_new_handle,
                                Socket::new(PeerAddress {
                                    id: us_id.clone(),
                                    handle: *us_handle,
                                }),
                            );
                            peers_listener
                                .nonconsumed_handles
                                .push_front(peer_new_handle);
                            peer_module.socketman.wakes.push(*peers_handle);

                            // Connect us to them
                            let us_new_handle = *us_next_handle;
                            *us_next_handle += 1;
                            us_sockets.insert(
                                us_new_handle,
                                Socket::new(PeerAddress {
                                    id: peer.clone(),
                                    handle: *peers_handle,
                                }),
                            );
                            peers_listener.nonconsumed_handles.push_front(us_new_handle);
                            us_module.socketman.wakes.push(*peers_handle);
                        }
                    }
                }
            }
        }

        us_module.module.wake(&mut us_module.socketman).unwrap();
    }
}

// TODO: Inbox/outbox should just be dequeues of bytes, so writes push_back and reads pop_front

fn main() -> Result<(), Box<dyn Error>> {
    println!("Loading instances...");
    let mut manager = Manager::new();
    manager.add_module(
        "plugin_a",
        Module::from_path("../plugin_a/target/wasm32-unknown-unknown/release/plugin_a.wasm")?,
    );
    manager.add_module(
        "plugin_b",
        Module::from_path("../plugin_b/target/wasm32-unknown-unknown/release/plugin_b.wasm")?,
    );

    println!("Running:");
    for _ in 0..30 {
        manager.run();
    }

    Ok(())
}
