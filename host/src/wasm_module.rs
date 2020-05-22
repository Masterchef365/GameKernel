use anyhow::Result;
use futures::channel::mpsc::Sender;
use futures::future::poll_fn;
use game_kernel::matchmaker::Request;
use game_kernel::socket::SocketManager;
use protocols::*;
use std::ffi::c_void;
use std::fs::File;
use std::io;
use std::io::Read;
use std::task::Context;
use std::task::Poll;
use wasmer_runtime::{func, imports, instantiate, Array, Ctx, Func, Instance, Memory, WasmPtr};

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

struct RuntimeSupply<'a, 'b> {
    cx: &'a mut Context<'b>,
    sockman: &'a mut SocketManager,
}

impl WasmModule {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let mut wasm = Vec::new();
        File::open(path)?.read_to_end(&mut wasm)?;
        Self::new(&wasm)
    }

    pub fn new(source: &[u8]) -> Result<Self> {
        let import_object = imports! {
            "env" => {
                "write" => func!(|ctx: &mut Ctx, handle: Handle, buf: WasmPtr<u8, Array>, len: u32| {
                    let (mem, rt) = unsafe { ctx.memory_and_data_mut::<RuntimeSupply<'static, 'static>>(0) };
                    Maybe::encode(rt.sockman.write(handle, buf.deref(mem, 0, len).unwrap(), rt.cx))
                }),

                "flush" => func!(|ctx: &mut Ctx, handle: Handle| {
                    let (mem, rt) = unsafe { ctx.memory_and_data_mut::<RuntimeSupply<'static, 'static>>(0) };
                    Maybe::encode(rt.sockman.flush(handle, rt.cx).map(|v| v.map(|_| 0)))
                }),

                "read" => func!(|ctx: &mut Ctx, handle: Handle, buf: WasmPtr<u8, Array>, len: u32| {
                    let (mem, rt) = unsafe { ctx.memory_and_data_mut::<RuntimeSupply<'static, 'static>>(0) };
                    Maybe::encode(rt.sockman.read(handle, buf.deref(mem, 0, len).unwrap(), rt.cx))
                }),

                "connect" => {
                    func!(|ctx: &mut Ctx, peer: WasmPtr<u8, Array>, len: u32, port: u16| {
                        let (mem, rt) = unsafe { ctx.memory_and_data_mut::<RuntimeSupply<'static, 'static>>(0) };
                        if let Ok(peer) = decode_string(mem, peer, len) {
                            Maybe::encode(rt.sockman.connect(&peer, port))
                        } else {
                            Maybe::encode(Poll::Ready(Err(io::Error::from(io::ErrorKind::InvalidData))))
                        }
                    })
                },

                "listener_create" => func!(|ctx: &mut Ctx, port: u16| {
                    let (_, rt) = unsafe { ctx.memory_and_data_mut::<RuntimeSupply<'static, 'static>>(0) };
                    Maybe::encode(rt.sockman.listener_create(port))
                }),

                "listen" => func!(|ctx: &mut Ctx, handle: Handle| {
                    let (_, rt) = unsafe { ctx.memory_and_data_mut::<RuntimeSupply<'static, 'static>>(0) };
                    Maybe::encode(rt.sockman.listen(handle, rt.cx))
                }),

                "close" => func!(|ctx: &mut Ctx, handle: Handle| {
                    let (_, rt) = unsafe { ctx.memory_and_data_mut::<RuntimeSupply<'static, 'static>>(0) };
                    rt.sockman.close(handle)
                }),

                "debug" => func!(|ctx: &mut Ctx, peer: WasmPtr<u8, Array>, len: u32| {
                    if let Ok(string) = decode_string(ctx.memory(0), peer, len) {
                        println!("Module debug: {}", string);
                    }
                }),
            },
        };

        let instance = instantiate(&source, &import_object).unwrap();

        let main_func: Func = instance.func("main")?;
        main_func.call().unwrap();

        Ok(Self { instance })
    }

    fn run(&mut self, sockman: &mut SocketManager, cx: &mut Context) {
        let runtime_supply = RuntimeSupply { sockman, cx };
        self.instance.context_mut().data = &runtime_supply as *const _ as *mut c_void;

        let wake_func: Func<u32, ()> = self.instance.func("wake").unwrap();

        for handle in sockman.wakes(cx) {
            wake_func.call(handle).unwrap();
        }

        let poll_func: Func = self.instance.func("run_tasks").unwrap();
        poll_func.call().expect("Wasm task failure");
    }

    pub async fn task(mut self, id: ModuleId, matchmaker: Sender<Request>) {
        let mut sockman = SocketManager::new(id.clone(), matchmaker);
        loop {
            poll_fn(|cx| {
                //eprintln!("\n************ {} ************", id);
                self.run(&mut sockman, cx);
                //eprintln!("\n************ END {} ************", id);
                Poll::<()>::Pending
            })
            .await;
        }
    }
}
