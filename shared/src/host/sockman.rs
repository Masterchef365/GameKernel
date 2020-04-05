use crate::{Maybe, Handle};
use std::cell::Cell;

pub trait SocketManager {
    fn connect(addr: &str, port: u16) -> Maybe;
    fn listener_create(port: u16) -> Maybe;
    fn listen(handle: Handle) -> Maybe;
    fn close(handle: Handle);

    fn read(handle: Handle, buffer: &[Cell<u8>]) -> Maybe;
    fn write(handle: Handle, buffer: &[Cell<u8>]) -> Maybe;

    fn wakes() -> Vec<Handle>;
}
