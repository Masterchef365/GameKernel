mod module;
use module::{Module, SocketManager};
use std::cell::Cell;
use std::error::Error;
use std::io;
use std::task::Poll;

use libplugin::Handle;

struct DebugSockMan;

impl SocketManager for DebugSockMan {
    fn connect(&mut self, addr: &str, port: u16) -> Poll<io::Result<Handle>> {
        println!("Connect {}:{}", addr, port);
        Poll::Ready(Ok(0))
    }

    fn listener_create(&mut self, port: u16) -> Poll<io::Result<Handle>> {
        println!("Listner create {}", port);
        Poll::Ready(Ok(0))
    }

    fn listen(&mut self, handle: Handle) -> Poll<io::Result<Handle>> {
        println!("Listen {}", handle);
        Poll::Ready(Ok(0))
    }

    fn close(&mut self, handle: Handle) {
        println!("Close {}", handle);
    }

    fn read(&mut self, handle: Handle, _buffer: &[Cell<u8>]) -> Poll<io::Result<u32>> {
        println!("Read {}", handle);
        Poll::Ready(Ok(0))
    }

    fn write(&mut self, handle: Handle, _buffer: &[Cell<u8>]) -> Poll<io::Result<u32>> {
        println!("Write {}", handle);
        Poll::Ready(Ok(0))
    }

    fn wakes(&mut self) -> Vec<Handle> {
        vec![]
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
    }

    Ok(())
}
