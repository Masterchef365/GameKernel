use futures::channel::mpsc::{channel, Receiver, SendError, Sender};
use futures::sink::SinkExt;
use futures::stream::{Stream, StreamExt};
use loopback::Loopback;
use protocols::*;
use std::collections::HashMap;

pub type MatchMakerConnection = Sender<Request>;
pub type ConnSender = Sender<Loopback>;

/// Connect to a module via MatchMaker
pub async fn connect(
    id: impl Into<ModuleId>,
    port: Port,
    matchmaker: &mut MatchMakerConnection,
) -> Result<Option<Loopback>, SendError> {
    let (dest_socket, mut socket) = channel(MATCHMAKER_MAX_REQ);
    matchmaker
        .send(Request {
            dest_socket,
            id: id.into(),
            port,
            conn_type: ConnType::Connector,
        })
        .await?;
    Ok(socket.next().await)
}

/// Create a new socket listener via MatchMaker
pub async fn create_listener(
    id: impl Into<ModuleId>,
    port: Port,
    matchmaker: &mut MatchMakerConnection,
) -> Result<impl Stream<Item = Loopback>, SendError> {
    let (dest_socket, socket) = channel(MATCHMAKER_MAX_REQ);
    matchmaker
        .send(Request {
            dest_socket,
            id: id.into(),
            port,
            conn_type: ConnType::Listener,
        })
        .await?;
    Ok(socket)
}

/// A request to the match maker
pub struct Request {
    /// Connector: Destination host; Listener: Host
    pub id: ModuleId,
    /// Broadcast or destination port
    pub port: Port,
    /// Connection type (listener, connector)
    pub conn_type: ConnType,
    /// Channel on which to receive connections
    pub dest_socket: ConnSender,
}

/// Connection type (listener, connector)
#[derive(Debug)]
pub enum ConnType {
    Connector,
    Listener,
}

/// Connection facilitator
pub struct MatchMaker {
    receiver: Receiver<Request>,
    active_connections: HashMap<(ModuleId, Port), Vec<ConnSender>>,
    listeners: HashMap<(ModuleId, Port), ConnSender>,
}

/// Match maker channel message limit
pub const MATCHMAKER_MAX_REQ: usize = 32;

impl MatchMaker {
    /// Create a new match maker, returning a channel which listens to requests
    pub fn new() -> (Self, MatchMakerConnection) {
        let (sender, receiver) = channel(MATCHMAKER_MAX_REQ);
        let instance = Self {
            receiver,
            active_connections: Default::default(),
            listeners: Default::default(),
        };
        (instance, sender)
    }

    /// The match maker loop, never returns and handles new connections through the
    /// MatchMakerConnection channel returned on creation.
    pub async fn task(mut self) {
        while let Some(msg) = self.receiver.next().await {
            match msg.conn_type {
                ConnType::Listener => self.new_listener(msg.id, msg.port, msg.dest_socket).await,
                ConnType::Connector => self.new_connector(msg.id, msg.port, msg.dest_socket).await,
            }
        }
        panic!("Matchmaker task ended!")
    }

    async fn new_connector(&mut self, id: ModuleId, port: Port, mut connector: ConnSender) {
        // Atempt to connect the socket immediately
        let addr = (id, port);
        if let Some(listener) = self.listeners.get_mut(&addr) {
            let (a, b) = Loopback::pair();
            if listener.send(a).await.is_ok() {
                // Note that we don't care about the return value, because if it failed to send
                // then the other side will notice when it is unable to send or receive
                let _ = connector.send(b).await;

                // Don't add this connector to our collection, as connectors are one-shot.
                return;
            } else {
                // Connection is hung up, never attempt to contact it again
                self.listeners.remove(&addr);
            }
        }

        // Slate this connector for connection as soon as the listener its looking for becomes
        // available.
        self.active_connections
            .entry(addr)
            .or_insert(vec![])
            .push(connector)
    }

    async fn new_listener(&mut self, id: ModuleId, port: Port, mut listener: ConnSender) {
        // If there's a connector list for the address of the connecting listener, try to create a
        // connector for each entry.
        let addr = (id, port);
        if let Some(connector_list) = self.active_connections.get_mut(&addr) {
            while let Some(mut connector) = connector_list.pop() {
                let (a, b) = Loopback::pair();

                if listener.send(b).await.is_err() {
                    connector_list.push(connector);
                    // Abort without adding the listener to the `listeners` collection.
                    return;
                }

                let _ = connector.send(a).await;
            }
        }
        self.listeners.insert(addr, listener);
    }
}
