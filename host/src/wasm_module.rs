use game_kernel::executor::Module;
use game_kernel::socket::SocketManager;
use game_kernel::maybe::Maybe;
use game_kernel::socket_types::*;
use std::ffi::c_void;
use std::fs::File;
use std::io;
use std::io::Read;
use std::task::Poll;
use wasmer_runtime::{func, imports, instantiate, Array, Ctx, Func, Instance, Memory, WasmPtr};

type Fallible<T> = Result<T, Box<dyn std::error::Error>>;

pub struct WasmModule {
    instance: Instance,
}

fn decode_string(
    mem: &Memory,
    arr: WasmPtr<u8, Array>,
    len: u32,
) -> Result<String, std::string::FromUtf8Error> {
    let peer = arr.deref(mem, 0, len).unwrap();
    String::from_utf8(peer.iter().map(|b| b.get()).collect())
}

impl WasmModule {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Fallible<Self> {
        let mut wasm = Vec::new();
        File::open(path)?.read_to_end(&mut wasm)?;
        Self::new(&wasm)
    }

    pub fn new(source: &[u8]) -> Fallible<Self> {
        let import_object = imports! {
            "env" => {
                "write" => func!(|ctx: &mut Ctx, handle: Handle, buf: WasmPtr<u8, Array>, len: u32| {
                    let (mem, sockman) = unsafe { ctx.memory_and_data_mut::<SocketManager>(0) };
                    Maybe::encode(sockman.write(handle, buf.deref(mem, 0, len).unwrap()))
                }),

                "read" => func!(|ctx: &mut Ctx, handle: Handle, buf: WasmPtr<u8, Array>, len: u32| {
                    let (mem, sockman) = unsafe { ctx.memory_and_data_mut::<SocketManager>(0) };
                    Maybe::encode(sockman.read(handle, buf.deref(mem, 0, len).unwrap()))
                }),

                "connect" => {
                    func!(|ctx: &mut Ctx, peer: WasmPtr<u8, Array>, len: u32, port: u16| {
                        let (mem, sockman) = unsafe { ctx.memory_and_data_mut::<SocketManager>(0) };
                        if let Ok(peer) = decode_string(mem, peer, len) {
                            Maybe::encode(sockman.connect(&peer, port))
                        } else {
                            Maybe::encode(Poll::Ready(Err(io::Error::new(io::ErrorKind::InvalidData, ""))))
                        }
                    })
                },

                "listener_create" => func!(|ctx: &mut Ctx, port: u16| {
                    let (_, sockman) = unsafe { ctx.memory_and_data_mut::<SocketManager>(0) };
                    Maybe::encode(sockman.listener_create(port))
                }),

                "listen" => func!(|ctx: &mut Ctx, handle: Handle| {
                    let (_, sockman) = unsafe { ctx.memory_and_data_mut::<SocketManager>(0) };
                    Maybe::encode(sockman.listen(handle))
                }),

                "close" => func!(|ctx: &mut Ctx, handle: Handle| {
                    let (_, sockman) = unsafe { ctx.memory_and_data_mut::<SocketManager>(0) };
                    sockman.close(handle)
                }),

                "debug" => func!(|ctx: &mut Ctx, peer: WasmPtr<u8, Array>, len: u32| {
                    if let Ok(string) = decode_string(ctx.memory(0), peer, len) {
                        println!("Module debug: {}", string);
                    }
                }),
            },
        };

        let instance = instantiate(&source, &import_object)?;

        let main_func: Func = instance.func("main")?;
        main_func.call()?;

        Ok(Self { instance })
    }
}

impl Module for WasmModule {
    fn run(&mut self, sockman: &mut SocketManager) {
        self.instance.context_mut().data = sockman as *mut _ as *mut c_void;

        let wake_func: Func<u32, ()> = self.instance.func("wake").unwrap();

        for handle in sockman.wakes() {
            wake_func.call(handle).unwrap();
        }

        let poll_func: Func = self.instance.func("run_tasks").unwrap();
        poll_func.call().unwrap();
    }
}
