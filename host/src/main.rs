use std::cell::Cell;
use std::collections::HashMap;
use std::error::Error;
use std::ffi::c_void;
use std::fs::File;
use std::io::{self, Read};
use std::sync::{Arc, RwLock};
use wasmer_runtime::{error, func, imports, instantiate, Array, Ctx, Func, Instance, WasmPtr};

type Fd = u32;
type Fallible<T> = Result<T, Box<dyn Error>>;

struct SocketManager {
    fd_counter: Fd,
}

impl SocketManager {
    pub fn new() -> Self {
        Self { fd_counter: 0 }
    }

    pub fn close(&mut self, fd: Fd) {
        todo!("Close {}", fd)
    }

    pub fn socket(&mut self) -> Fd {
        let ret = self.fd_counter;
        self.fd_counter += 1;
        ret
    }

    pub fn read(&mut self, fd: Fd, buf: &[Cell<u8>]) -> i64 {
        for val in buf {
            val.set(b'W');
        }
        buf.len() as i64
    }

    pub fn write(&mut self, fd: Fd, buf: &[Cell<u8>]) -> i64 {
        for val in buf {
            print!("{}", val.get() as char);
        }
        println!();
        buf.len() as i64
    }
}

struct ModuleManager {
    socket_manager: Arc<RwLock<SocketManager>>,
    instance: Instance,
}

impl ModuleManager {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Fallible<Self> {
        let mut wasm = Vec::new();
        File::open(path)?.read_to_end(&mut wasm)?;
        Self::new(&wasm)
    }

    fn get_socketmanager(ctx: &mut Ctx) -> &mut SocketManager {
        unsafe {
            let (_, socket) = ctx.memory_and_data_mut::<Box<SocketManager>>(0);
            socket
        }
    }

    pub fn new(source: &[u8]) -> Fallible<Self> {
        let socket_manager = Arc::new(RwLock::new(SocketManager::new()));

        let import_object = imports! {
            "env" => {
                "write" => {
                    let socket_manager = socket_manager.clone();
                    func!(move |ctx: &mut Ctx, fd: Fd, buf: WasmPtr<u8, Array>, len: u32| {
                        let buf = buf.deref(ctx.memory(0), 0, len).unwrap();
                        socket_manager.write().unwrap().write(fd, buf)
                    })
                },

                "socket" => {
                    let socket_manager = socket_manager.clone();
                    func!(move |ctx: &mut Ctx, ptr: WasmPtr<u8, Array>, len: u32| {
                        socket_manager.write().unwrap().socket()
                    })
                },

                "read" => {
                    let socket_manager = socket_manager.clone();
                    func!(move |ctx: &mut Ctx, fd: Fd, buf: WasmPtr<u8, Array>, len: u32| {
                        let buf = buf.deref(ctx.memory(0), 0, len).unwrap();
                        socket_manager.write().unwrap().read(fd, buf)
                    })
                },

                "close" => {
                    let socket_manager = socket_manager.clone();
                    func!(move |ctx: &mut Ctx, fd: Fd| {
                        socket_manager.write().unwrap().close(fd)
                    })
                },
            },
        };

        let mut instance = instantiate(&source, &import_object)?;

        Ok(Self {
            instance,
            socket_manager,
        })
    }

    pub fn run(&mut self) -> Fallible<()> {
        let main_func: Func = self.instance.func("main")?;
        main_func.call()?;
        Ok(())
    }
}

const WASM_PATH: &str = "../plugin/target/wasm32-unknown-unknown/release/plugin.wasm";
fn main() -> Result<(), Box<dyn Error>> {
    println!("Loading instance...");
    let mut manager = ModuleManager::from_path(WASM_PATH)?;

    println!("Executing...");
    manager.run()?;

    Ok(())
}
