use crate::module::{Fallible, Module};
use crate::socket::SocketManager;
use libloading as lib;

pub struct NativeModule {
    instance: lib::Library,
}

impl NativeModule {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Fallible<Self> {
        let instance = lib::Library::new(path.as_ref())?;
        unsafe {
            let main: lib::Symbol<extern "C" fn()> = instance.get(b"main")?;
            main();
        }
        Ok(Self { instance })
    }
}

impl Module for NativeModule {
    fn wake(&mut self, sockman: &mut SocketManager) -> Fallible<()> {
        unsafe {
            let set_socketmanager: lib::Symbol<extern "C" fn(&mut SocketManager)> =
                self.instance.get(b"set_socketmanager")?;
            let wake: lib::Symbol<extern "C" fn(u32)> = self.instance.get(b"wake")?;
            let poll_func: lib::Symbol<extern "C" fn()> = self.instance.get(b"run_tasks")?;

            set_socketmanager(sockman);
            for handle in sockman.wakes() {
                wake(handle);
            }
            poll_func();
        }
        Ok(())
    }
}
