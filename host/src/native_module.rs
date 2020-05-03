use game_kernel::executor::Module;
use game_kernel::socket::SocketManager;
use libloading as lib;

type Fallible<T> = Result<T, Box<dyn std::error::Error>>;

pub struct Calls<'instance> {
    pub set_socketmanager: lib::Symbol<'instance, extern "C" fn(&mut SocketManager)>,
    pub wake: lib::Symbol<'instance, extern "C" fn(u32)>,
    pub poll_func: lib::Symbol<'instance, extern "C" fn()>,
}

rental! {
    pub mod nm {
        use super::*;

        #[rental]
        pub struct NativeModule {
            instance: Box<libloading::Library>,
            calls: Calls<'instance>,
        }
    }
}

pub use nm::NativeModule;

impl NativeModule {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self, Box<libloading::Error>> {
        let instance = lib::Library::new(path.as_ref())?;
        unsafe {
            let main: lib::Symbol<extern "C" fn()> = instance.get(b"main")?;
            main();

            NativeModule::try_new(Box::new(instance), |instance| {
                Ok(Calls {
                    set_socketmanager: instance.get(b"set_socketmanager")?,
                    wake: instance.get(b"wake")?,
                    poll_func: instance.get(b"run_tasks")?,
                })
            })
            .map_err(|e: rental::RentalError<libloading::Error, _>| e.0.into())
        }
    }

    pub fn run(&mut self, sockman: &mut SocketManager) -> Fallible<()> {
        self.rent(|rent| {
            (rent.set_socketmanager)(sockman);
            for handle in sockman.wakes() {
                (rent.wake)(handle);
            }
            (rent.poll_func)();
            Ok(())
        })
    }
}

impl Module for NativeModule {
    fn run(&mut self, sockman: &mut SocketManager) {
        self.run(sockman).unwrap();
    }
}
