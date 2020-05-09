use crate::socket_types::*;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use futures::executor::ThreadPool;

pub struct Executor {
    pool: ThreadPool,
    sender: Sender<MatchMakerRequest>,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            pool: ThreadPool::new().expect("Failed to create threadpool"),
            sender: match_sender,
        }
    }
}
