use crate::{Maybe, Handle};
use std::cell::Cell;

/// Thin wrapper over syscalls from the module to the host
pub trait SocketManager {
    fn connect(&mut self, addr: &str, port: u16) -> Maybe;
    fn listener_create(&mut self, port: u16) -> Maybe;
    fn listen(&mut self, handle: Handle) -> Maybe;
    fn close(&mut self, handle: Handle);

    fn read(&mut self, handle: Handle, buffer: &[Cell<u8>]) -> Maybe;
    fn write(&mut self, handle: Handle, buffer: &[Cell<u8>]) -> Maybe;

    fn wakes(&mut self) -> Vec<Handle>;
}
