use crate::socket::{ListenerType, ModuleId, PeerAddress, Socket, SocketManager};
use crate::wasm_module::WasmModule;
use std::collections::HashMap;
use crate::module::Module;

struct ManagedInstance {
    pub module: Box<dyn Module>,
    pub socketman: SocketManager,
}

pub struct Manager {
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

    pub fn add_module(&mut self, name: impl Into<ModuleId>, module: Box<dyn Module>) {
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
        (id, module): (&ModuleId, &mut ManagedInstance),
        others: &mut HashMap<ModuleId, ManagedInstance>,
    ) {
        // Packet routing
        for socket in &mut module.socketman.sockets.values_mut() {
            if !socket.outbox.is_empty() {
                if let Some(peer) = others.get_mut(&socket.peer.id) {
                    if let Some(peer_socket) = peer.socketman.sockets.get_mut(&socket.peer.handle) {
                        peer_socket.inbox.extend(socket.outbox.drain(..));
                        peer.socketman.wakes.push(socket.peer.handle);
                    } else {
                        eprintln!("Err: Handle not found");
                    }
                } else {
                    eprintln!("Err: Peer not found");
                }
            }
        }

        // Connection handling
        let listeners = &mut module.socketman.listeners;
        let next_handle = &mut module.socketman.next_handle;
        let sockets = &mut module.socketman.sockets;

        for listener in listeners.values_mut() {
            if let ListenerType::Client(peer, port) = &listener.listener_type {
                if let Some(peer_module) = others.get_mut(peer) {
                    let peer_listeners = &mut peer_module.socketman.listeners;
                    let peer_next_handle = &mut peer_module.socketman.next_handle;
                    let peer_sockets = &mut peer_module.socketman.sockets;

                    for (peers_handle, peers_listener) in peer_listeners {
                        if peers_listener.listener_type == ListenerType::Server(*port) {
                            // Create handles
                            let new_handle = *next_handle;
                            *next_handle += 1;

                            let peer_new_handle = *peer_next_handle;
                            *peer_next_handle += 1;

                            // Connect them to us
                            peer_sockets.insert(
                                peer_new_handle,
                                Socket::new(PeerAddress {
                                    id: id.clone(),
                                    handle: new_handle,
                                }),
                            );
                            peers_listener
                                .nonconsumed_handles
                                .push_front(peer_new_handle);
                            peer_module.socketman.wakes.push(*peers_handle);

                            // Connect us to them
                            sockets.insert(
                                new_handle,
                                Socket::new(PeerAddress {
                                    id: peer.clone(),
                                    handle: peer_new_handle,
                                }),
                            );
                            listener.nonconsumed_handles.push_front(new_handle);
                            module.socketman.wakes.push(*peers_handle);
                        }
                    }
                }
            }
        }

        module.module.wake(&mut module.socketman).unwrap();
    }
}
