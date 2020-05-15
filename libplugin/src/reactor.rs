use protocols::Handle;
use once_cell::unsync::Lazy;
use std::collections::HashMap;
use std::task::Waker;

pub static mut REACTOR: Lazy<Reactor> = Lazy::new(Reactor::new);

#[no_mangle]
pub unsafe extern "C" fn wake(handle: Handle) {
    REACTOR.wake(handle);
}

pub struct Reactor {
    wakers: HashMap<Handle, Waker>,
}

impl Reactor {
    pub fn new() -> Self {
        Reactor {
            wakers: HashMap::new(),
        }
    }

    pub fn register(&mut self, handle: Handle, waker: Waker) {
        self.wakers.insert(handle, waker);
    }

    // TODO: Use this from run_tasks and do it in one batch!
    pub fn wake(&mut self, handle: Handle) {
        if let Some(waker) = self.wakers.remove(&handle) {
            waker.wake();
        }
    }
}

pub fn register(handle: Handle, waker: Waker) {
    unsafe {
        REACTOR.register(handle, waker);
    }
}
