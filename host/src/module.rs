use libplugin::{Handle, Maybe};
use std::cell::Cell;
use std::ffi::c_void;
use std::fs::File;
use std::io;
use std::io::Read;
use std::task::Poll;
use std::marker::PhantomData;
use wasmer_runtime::{func, imports, instantiate, Array, Ctx, Func, Instance, Memory, WasmPtr};

type Fallible<T> = Result<T, Box<dyn std::error::Error>>;

pub struct Module<S> {
    instance: Instance,
    _phantomdata: PhantomData<S>,
}

/// Thin wrapper over syscalls from the module to the host
pub trait WasmSocket {
    /// Initiate a new connection to a peer. Returns a handle that may be passed to listen().
    fn connect(&mut self, addr: &str, port: u16) -> Poll<io::Result<Handle>>;

    /// Create a new listener for a port. Calling this will create a listener that may be passed to
    /// listen()
    fn listener_create(&mut self, port: u16) -> Poll<io::Result<Handle>>;

    /// Listen for a new connection on this handle. 
    fn listen(&mut self, handle: Handle) -> Poll<io::Result<Handle>>;

    /// Close this handle
    fn close(&mut self, handle: Handle);

    /// Read from this handle
    fn read(&mut self, handle: Handle, buffer: &[Cell<u8>]) -> Poll<io::Result<u32>>;

    /// Write to this handle
    fn write(&mut self, handle: Handle, buffer: &[Cell<u8>]) -> Poll<io::Result<u32>>;

    /// Return the handles that are supposed to be awake
    fn wakes(&mut self) -> Vec<Handle>;
}

fn decode_string(
    mem: &Memory,
    arr: WasmPtr<u8, Array>,
    len: u32,
) -> Result<String, std::string::FromUtf8Error> {
    let peer = arr.deref(mem, 0, len).unwrap();
    String::from_utf8(peer.iter().map(|b| b.get()).collect())
}

impl<S: WasmSocket> Module<S> {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Fallible<Self> {
        let mut wasm = Vec::new();
        File::open(path)?.read_to_end(&mut wasm)?;
        Self::new(&wasm)
    }

    pub fn new(source: &[u8]) -> Fallible<Self> {
        let import_object = imports! {
            "env" => {
                "write" => func!(|ctx: &mut Ctx, handle: Handle, buf: WasmPtr<u8, Array>, len: u32| {
                    let (mem, sockman) = unsafe { ctx.memory_and_data_mut::<S>(0) };
                    Maybe::encode(sockman.write(handle, buf.deref(mem, 0, len).unwrap()))
                }),

                "read" => func!(|ctx: &mut Ctx, handle: Handle, buf: WasmPtr<u8, Array>, len: u32| {
                    let (mem, sockman) = unsafe { ctx.memory_and_data_mut::<S>(0) };
                    Maybe::encode(sockman.read(handle, buf.deref(mem, 0, len).unwrap()))
                }),

                "connect" => {
                    func!(|ctx: &mut Ctx, peer: WasmPtr<u8, Array>, len: u32, port: u16| {
                        let (mem, sockman) = unsafe { ctx.memory_and_data_mut::<S>(0) };
                        if let Ok(peer) = decode_string(mem, peer, len) {
                            Maybe::encode(sockman.connect(&peer, port))
                        } else {
                            Maybe::encode(Poll::Ready(Err(io::Error::new(io::ErrorKind::InvalidData, ""))))
                        }
                    })
                },

                "listener_create" => func!(|ctx: &mut Ctx, port: u16| {
                    let (_, sockman) = unsafe { ctx.memory_and_data_mut::<S>(0) };
                    Maybe::encode(sockman.listener_create(port))
                }),

                "listen" => func!(|ctx: &mut Ctx, handle: Handle| {
                    let (_, sockman) = unsafe { ctx.memory_and_data_mut::<S>(0) };
                    Maybe::encode(sockman.listen(handle))
                }),

                "close" => func!(|ctx: &mut Ctx, handle: Handle| {
                    let (_, sockman) = unsafe { ctx.memory_and_data_mut::<S>(0) };
                    sockman.close(handle)
                }),

                "debug" => func!(|ctx: &mut Ctx, peer: WasmPtr<u8, Array>, len: u32| {
                    if let Ok(string) = decode_string(ctx.memory(0), peer, len) {
                        //println!("Module debug: {}", string);
                    }
                }),
            },
        };

        let instance = instantiate(&source, &import_object)?;

        let main_func: Func = instance.func("main")?;
        main_func.call()?;

        Ok(Self { instance, _phantomdata: PhantomData })
    }

    pub fn wake(&mut self, sockman: &mut S) -> Fallible<()> {
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
