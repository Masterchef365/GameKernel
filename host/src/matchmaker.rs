use crate::socket_types::*;
use futures::channel::mpsc::{channel, Receiver, SendError, Sender};
use futures::sink::SinkExt;
use futures::stream::{Stream, StreamExt};
use std::collections::HashMap;

pub type MMSender = Sender<Request>;

/// Connect to a module via MatchMaker
pub async fn connect(
    id: impl Into<ModuleId>,
    port: Port,
    matchmaker: &mut MMSender,
) -> Result<Option<TwoWayConnection>, SendError> {
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
    matchmaker: &mut MMSender,
) -> impl Stream<Item = TwoWayConnection> {
    let (dest_socket, mut socket) = channel(MATCHMAKER_MAX_REQ);
    matchmaker.send(Request {
        dest_socket,
        id: id.into(),
        port,
        conn_type: ConnType::Listener,
    });
    socket
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
    pub dest_socket: Sender<TwoWayConnection>,
}

/// Connection type (listener, connector)
pub enum ConnType {
    Connector,
    Listener,
}

/// Connection facilitator
pub struct MatchMaker {
    receiver: Receiver<Request>,
    active_connections: HashMap<(ModuleId, Port), Sender<TwoWayConnection>>,
    listeners: HashMap<(ModuleId, Port), Sender<TwoWayConnection>>,
}

pub const MATCHMAKER_MAX_REQ: usize = 90;

impl MatchMaker {
    /// Create a new match maker, returning a channel which listens to requests
    pub fn new() -> (Self, MMSender) {
        let (sender, receiver) = channel(MATCHMAKER_MAX_REQ);
        let instance = Self {
            receiver,
            active_connections: Default::default(),
            listeners: Default::default(),
        };
        (instance, sender)
    }

    /// The match maker loop
    pub async fn task(mut self) {
        while let Some(mut msg) = self.receiver.next().await {
            let addr = (msg.id, msg.port);

            let peer = match &msg.conn_type {
                ConnType::Listener => &mut self.active_connections,
                ConnType::Connector => &mut self.listeners,
            }
            .get_mut(&addr);

            if let Some(peer) = peer {
                let (a, b) = TwoWayConnection::pair();
                peer.send(a).await.unwrap();
                msg.dest_socket.send(b).await.unwrap();
                //TODO: Try to send it, and if you can't then remove it from the hashmap. This means the
                //only other end has disconnected, which will be the module.
                continue;
            }

            match msg.conn_type {
                ConnType::Listener => &mut self.listeners,
                ConnType::Connector => &mut self.active_connections,
            }
            .insert(addr, msg.dest_socket);
        }
        panic!("Matchmaker task ended!")
    }
}
