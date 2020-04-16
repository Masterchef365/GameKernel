use crate::socket::SocketManager;

pub type Fallible<T> = Result<T, Box<dyn std::error::Error>>;

/// Any module that can be woken with a socket manager
pub trait Module {
    fn wake(&mut self, sockman: &mut SocketManager) -> Fallible<()>;
}
