use crate::Handle;
use std::task::Poll;
use std::cell::Cell;
use std::io;

/// Thin wrapper over syscalls from the module to the host
pub trait SocketManager {
    fn connect(&mut self, addr: &str, port: u16) -> Poll<io::Result<Handle>>;
    fn listener_create(&mut self, port: u16) -> Poll<io::Result<Handle>>;
    fn listen(&mut self, handle: Handle) -> Poll<io::Result<Handle>>;
    fn close(&mut self, handle: Handle);

    fn read(&mut self, handle: Handle, buffer: &[Cell<u8>]) -> Poll<io::Result<u32>>;
    fn write(&mut self, handle: Handle, buffer: &[Cell<u8>]) -> Poll<io::Result<u32>>;

    fn wakes(&mut self) -> Vec<Handle>;
}
