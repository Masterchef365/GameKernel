use std::cell::Cell;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{Read, self};
use std::sync::{Arc, RwLock};
use wasmer_runtime::{error, func, imports, instantiate, Array, Ctx, Func, Instance, WasmPtr};

use std::task::Poll;
pub type Maybe = i64;

fn main() {}
/*
struct Module {
    socket_manager: Arc<RwLock<SocketManager>>,
    instance: Instance,
}

impl Module {
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
                    func!(move |ctx: &mut Ctx, peer: WasmPtr<u8, Array>, len: u32| {
                        let peer = peer.deref(ctx.memory(0), 0, len).unwrap();
                        let peer = String::from_utf8(peer.iter().map(|b| b.get()).collect());
                        if let Ok(peer) = peer {
                            socket_manager.write().unwrap().socket(peer)
                        } else {
                            todo!("Error case for non-utf8 peer address")
                        }
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

                "debug_ex" => func!(|v: u32| println!("DEBUG: {}", v)),
            },
        };

        let mut instance = instantiate(&source, &import_object)?;

        let main_func: Func = instance.func("main")?;
        main_func.call()?;

        Ok(Self {
            instance,
            socket_manager,
        })
    }

    pub fn poll_complete(&mut self) -> Fallible<()> {
        let main_func: Func = self.instance.func("poll")?;
        main_func.call()?;
        Ok(())
    }
}

const WASM_PATH: &str = "../plugin/target/wasm32-unknown-unknown/release/plugin.wasm";
fn main() -> Result<(), Box<dyn Error>> {
    println!("Loading instance...");
    let mut manager = Module::from_path(WASM_PATH)?;

    println!("ONE");
    manager.poll_complete()?;
    println!("TWO");
    manager.poll_complete()?;
    println!("THREE");
    manager.poll_complete()?;

    Ok(())
}
*/
