use std::cell::Cell;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::sync::{Arc, RwLock};
use wasmer_runtime::{error, func, imports, instantiate, Array, Ctx, Func, Instance, WasmPtr};

type Fd = u32;
type Fallible<T> = Result<T, Box<dyn Error>>;

struct Mailbox {
    peer: String,
}

impl Mailbox {
    pub fn new(peer: String) -> Self {
        Self { peer }
    }
}

struct SocketManager {
    fd_counter: Fd,
    mailboxes: HashMap<Fd, Mailbox>,
}

impl SocketManager {
    pub fn new() -> Self {
        Self {
            fd_counter: 0,
            mailboxes: HashMap::new(),
        }
    }

    pub fn close(&mut self, fd: Fd) {
        self.mailboxes.remove(&fd);
    }

    pub fn socket(&mut self, peer: String) -> Fd {
        // Duplicate this socket handle if there's already a connection to that peer
        // TODO: Maybe throw and error instead?
        if let Some(dup) =
            self.mailboxes
                .iter()
                .find_map(|(fd, mail)| if mail.peer == peer { Some(fd) } else { None })
        {
            return *dup;
        }

        let fd = self.fd_counter;
        self.mailboxes.insert(fd, Mailbox::new(peer));
        self.fd_counter += 1;
        fd
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

    println!("Executing...");
    manager.poll_complete()?;

    Ok(())
}
