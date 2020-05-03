use crate::executor::{WakeRequest, WakeRequestBody};
use crate::socket_types::*;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};

pub struct MatchListener {
    pub module: ModuleId,
    pub handle: Handle,
    pub port: Port,
}

pub struct MatchConnector {
    pub module: ModuleId,
    pub handle: Handle,
    pub dest_module: ModuleId,
    pub dest_port: Port,
}

pub enum MatchRequest {
    Listener(MatchListener),
    Connector(MatchConnector),
}

pub struct MatchMaker {
    listening: HashMap<(ModuleId, Port), Handle>,
    connecting: Vec<MatchConnector>,
    receiver: Receiver<MatchRequest>,
    executor_waker: Sender<WakeRequest>,
}

impl MatchMaker {
    pub fn new(receiver: Receiver<MatchRequest>, executor_waker: Sender<WakeRequest>) -> Self {
        Self {
            listening: HashMap::new(),
            connecting: Vec::new(),
            receiver,
            executor_waker,
        }
    }

    pub fn run(&mut self) {
        for request in self.receiver.try_iter() {
            match request {
                MatchRequest::Listener(l) => {
                    self.listening.insert((l.module, l.port), l.handle);
                }
                MatchRequest::Connector(c) => self.connecting.push(c),
            }
        }

        let listening = &mut self.listening;
        let executor_waker = &mut self.executor_waker;
        self.connecting.retain(|connection| {
            if let Some(listener_handle) =
                listening.remove(&(connection.dest_module.clone(), connection.dest_port))
            {
                let (a, b) = TwoWayConnection::pair();
                executor_waker.send(WakeRequest {
                    module: connection.module.clone(),
                    body: WakeRequestBody::EndPointConnected(connection.handle, a),
                });
                executor_waker.send(WakeRequest {
                    module: connection.dest_module.clone(),
                    body: WakeRequestBody::EndPointConnected(listener_handle, b),
                });
                false
            } else {
                true
            }
        });
    }
}
