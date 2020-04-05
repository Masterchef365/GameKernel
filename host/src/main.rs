use std::cell::Cell;
use std::error::Error;
use std::ffi::c_void;
use std::fs::File;
use std::io::Read;
use wasmer_runtime::{func, imports, instantiate, Array, Ctx, Func, Instance, WasmPtr};

use shared::{Handle, Maybe, SocketManager};

type Fallible<T> = Result<T, Box<dyn std::error::Error>>;

struct Module {
    instance: Instance,
}

impl Module {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Fallible<Self> {
        let mut wasm = Vec::new();
        File::open(path)?.read_to_end(&mut wasm)?;
        Self::new(&wasm)
    }

    pub fn new(source: &[u8]) -> Fallible<Self> {
        let import_object = imports! {
            "env" => {
                "write" => func!(|ctx: &mut Ctx, handle: Handle, buf: WasmPtr<u8, Array>, len: u32| {
                    let (mem, sockman) = unsafe { ctx.memory_and_data_mut::<&mut Box<dyn SocketManager>>(0) };
                    sockman.write(handle, buf.deref(mem, 0, len).unwrap()).0
                }),

                "read" => func!(|ctx: &mut Ctx, handle: Handle, buf: WasmPtr<u8, Array>, len: u32| {
                    let (mem, sockman) = unsafe { ctx.memory_and_data_mut::<&mut Box<dyn SocketManager>>(0) };
                    sockman.read(handle, buf.deref(mem, 0, len).unwrap()).0
                }),

                "connect" => {
                    func!(|ctx: &mut Ctx, peer: WasmPtr<u8, Array>, len: u32, port: u16| {
                        let (mem, sockman) = unsafe { ctx.memory_and_data_mut::<&mut Box<dyn SocketManager>>(0) };
                        let peer = peer.deref(mem, 0, len).unwrap();
                        let peer = String::from_utf8(peer.iter().map(|b| b.get()).collect());
                        if let Ok(peer) = peer {
                            sockman.connect(&peer, port).0
                        } else {
                            todo!("Error case for non-utf8 peer address")
                        }
                    })
                },

                "listener_create" => func!(|ctx: &mut Ctx, port: u16| {
                    let (_, sockman) = unsafe { ctx.memory_and_data_mut::<&mut Box<dyn SocketManager>>(0) };
                    sockman.listener_create(port).0
                }),

                "listen" => func!(|ctx: &mut Ctx, handle: Handle| {
                    let (_, sockman) = unsafe { ctx.memory_and_data_mut::<&mut Box<dyn SocketManager>>(0) };
                    sockman.listen(handle).0
                }),

                "close" => func!(|ctx: &mut Ctx, handle: Handle| {
                    let (_, sockman) = unsafe { ctx.memory_and_data_mut::<&mut Box<dyn SocketManager>>(0) };
                    sockman.close(handle)
                }),
            },
        };

        let instance = instantiate(&source, &import_object)?;

        let main_func: Func = instance.func("main")?;
        main_func.call()?;

        Ok(Self { instance })
    }

    pub fn wake(&mut self, sockman: &mut Box<dyn SocketManager>) -> Fallible<()> {
        self.instance.context_mut().data = sockman as *mut _ as *mut c_void;

        let wake_func: Func<u32, ()> = self.instance.func("wake")?;
        for handle in sockman.wakes() {
            wake_func.call(handle)?;
        }

        let poll_func: Func = self.instance.func("run_tasks")?;
        poll_func.call()?;
        Ok(())
    }
}

struct DebugSockMan;

impl SocketManager for DebugSockMan {
    fn connect(&mut self, addr: &str, port: u16) -> Maybe {
        println!("Connect {}:{}", addr, port);
        Maybe(0)
    }

    fn listener_create(&mut self, port: u16) -> Maybe {
        println!("Listner create {}", port);
        Maybe(0)
    }

    fn listen(&mut self, handle: Handle) -> Maybe {
        println!("Listen create {}", handle);
        Maybe(0)
    }

    fn close(&mut self, handle: Handle) {
        println!("Close {}", handle);
    }

    fn read(&mut self, handle: Handle, _buffer: &[Cell<u8>]) -> Maybe {
        println!("Read {}", handle);
        Maybe(0)
    }

    fn write(&mut self, handle: Handle, _buffer: &[Cell<u8>]) -> Maybe {
        println!("Write {}", handle);
        Maybe(0)
    }

    fn wakes(&mut self) -> Vec<Handle> {
        println!("Wakes");
        Vec::new()
    }
}

const WASM_PATH: &str = "../plugin/target/wasm32-unknown-unknown/debug/plugin.wasm";
fn main() -> Result<(), Box<dyn Error>> {
    println!("Loading instance...");
    let mut manager = Module::from_path(WASM_PATH)?;
    let mut sockman: Box<dyn SocketManager> = Box::new(DebugSockMan);

    println!("Running:");
    for _ in 0..10 {
        manager.wake(&mut sockman)?;
        println!();
    }

    Ok(())
}
