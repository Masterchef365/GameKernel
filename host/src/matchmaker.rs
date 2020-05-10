use crate::socket_types::*;
use std::collections::HashMap;
use futures::channel::mpsc::{Receiver, Sender, channel};
use futures::stream::StreamExt;
use futures::sink::SinkExt;

pub enum MatchMakerRequestBody {
    Connector,
    Listener,
}

pub struct MatchMakerRequest {
    pub id: ModuleId,
    pub port: Port,
    pub body: MatchMakerRequestBody,
    pub dest_socket: Sender<TwoWayConnection>,
}

pub struct MatchMaker {
    receiver: Receiver<MatchMakerRequest>,
    active_connections: HashMap<(ModuleId, Port), Sender<TwoWayConnection>>,
    listeners: HashMap<(ModuleId, Port), Sender<TwoWayConnection>>,
}

pub const MATCHMAKER_MAX_REQ: usize = 90;
impl MatchMaker {
    pub fn new() -> (Self, Sender<MatchMakerRequest>) {
        let (sender, receiver) = channel(MATCHMAKER_MAX_REQ);
        let instance = Self {
            receiver,
            active_connections: Default::default(),
            listeners: Default::default(),
        };
        (instance, sender)
    }

    pub async fn task(mut self) {
        while let Some(mut msg) = self.receiver.next().await {
            let addr = (msg.id, msg.port);

            let peer = match &msg.body {
                MatchMakerRequestBody::Listener => &mut self.active_connections,
                MatchMakerRequestBody::Connector => &mut self.listeners,
            }.get_mut(&addr);

            if let Some(peer) = peer {
                let (a, b) = TwoWayConnection::pair();
                peer.send(a).await.unwrap();
                msg.dest_socket.send(b).await.unwrap();
                //TODO: If the peer was a connection, remove it!
                //Try to send it, and if you can't then remove it from the hashmap. This means the
                //only other end has disconnected, which will be the module.
                continue;
            }

            match msg.body {
                MatchMakerRequestBody::Listener => &mut self.listeners,
                MatchMakerRequestBody::Connector => &mut self.active_connections,
            }.insert(addr, msg.dest_socket);
        }
        panic!("Matchmaker task ended!")
    }
}
