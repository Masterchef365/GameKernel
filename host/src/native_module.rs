use crate::module::{Module, Fallible};
use crate::socket::SocketManager;
use std::future::Future;

pub struct NativeModule {
    task_pool: LocalPool,
}

impl NativeModule {
    pub fn new() -> Self {
        Self {
            task_pool: LocalPool::new(),
        }
    }

    pub fn spawn(&mut self, f: impl Future<Output = ()> + 'static) {
        self.task_pool.spawner().spawn_local(f).unwrap();
    }
}

impl Module for NativeModule {
    fn wake(&mut self, sockman: &mut SocketManager) -> Fallible<()> {
        self.task_pool.run_until_stalled();
        Ok(())
    }
}
