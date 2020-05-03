use crate::matchmaker::MatchRequest;
use crate::socket::SocketManager;
use crate::socket_types::*;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};

pub trait Module {
    fn run(&mut self, socket_manager: &mut SocketManager);
}

pub enum WakeRequestBody {
    EndPointConnected(Handle, TwoWayConnection),
    Data(Handle),
}

pub struct WakeRequest {
    pub module: ModuleId,
    pub body: WakeRequestBody,
}

pub struct Executor {
    wake_sender: Sender<WakeRequest>,
    wake_receiver: Receiver<WakeRequest>,
    match_sender: Sender<MatchRequest>,
    modules: HashMap<ModuleId, ManagedModule>,
}

struct ManagedModule {
    socket_manager: SocketManager,
    module: Box<dyn Module>,
}

impl Executor {
    pub fn new(match_sender: Sender<MatchRequest>) -> Self {
        let (wake_sender, wake_receiver) = channel();
        Self {
            wake_sender,
            wake_receiver,
            match_sender,
            modules: HashMap::new(),
        }
    }

    pub fn sender(&self) -> Sender<WakeRequest> {
        self.wake_sender.clone()
    }

    pub fn add_module(&mut self, id: ModuleId, module: Box<dyn Module>) {
        self.modules.insert(
            id.clone(),
            ManagedModule {
                socket_manager: SocketManager::new(
                    self.wake_sender.clone(),
                    self.match_sender.clone(),
                    id,
                ),
                module,
            },
        );
    }

    pub fn run(&mut self) {
        for wake_req in self.wake_receiver.try_iter() {
            if let Some(module) = self.modules.get_mut(&wake_req.module) {
                module.socket_manager.wake(wake_req.body);
            }
        }

        for module in self.modules.values_mut() {
            module.module.run(&mut module.socket_manager);
        }
    }
}
