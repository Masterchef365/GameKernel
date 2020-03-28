use once_cell::unsync::Lazy;
use std::collections::HashMap;
use std::task::Waker;
use crate::Handle;

pub static mut REACTOR: Lazy<Reactor> = Lazy::new(Reactor::new);

pub struct Reactor {
    wakers: HashMap<Handle, Waker>,
}

impl Reactor {
    pub fn new() -> Self {
        Reactor {
            wakers: HashMap::new(),
        }
    }

    pub fn register(&mut self, fd: Handle, waker: Waker) {
        self.wakers.insert(fd, waker);
    }

    pub fn wake_fd(&mut self, fd: Handle) {
        if let Some(waker) = self.wakers.remove(&fd) {
            waker.wake();
        }
    }
}

// TODO: Move this into run_tasks somehow!
#[no_mangle]
pub unsafe extern "C" fn wake_fd(fd: Handle) {
    REACTOR.wake_fd(fd);
}


pub fn register(fd: Handle, waker: Waker) {
    unsafe {
        REACTOR.register(fd, waker);
    }
}
