// TODO: Feature gate these
mod plugin;
pub use plugin::*;

mod host;
pub use host::*;

use std::io::{self, ErrorKind};
use std::task::Poll;

pub type Handle = u32;

/// Either represents an error, or a u32
#[repr(transparent)]
pub struct Maybe(pub i64);

impl Maybe {
    pub fn errorkind(&self) -> Result<u32, io::ErrorKind> {
        match self.0 {
            e if e >= 0 => Ok(e as u32),
            -1 => Err(ErrorKind::WouldBlock),
            -2 => Err(ErrorKind::AlreadyExists),
            _ => Err(ErrorKind::Other),
        }
    }

    pub fn into_poll(self) -> Poll<io::Result<u32>> {
        match self.errorkind() {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(ErrorKind::WouldBlock) => Poll::Pending,
            Err(k) => Poll::Ready(Err(io::Error::from(k))),
        }
    }

    pub fn encode(poll: Poll<io::Result<u32>>) -> i64 {
        Self::from(poll).0
    }
}

impl From<Poll<io::Result<u32>>> for Maybe {
    fn from(poll: Poll<io::Result<u32>>) -> Self {
        Maybe(match poll {
            Poll::Ready(Ok(n)) => n as i64,
            Poll::Pending => -1,
            Poll::Ready(Err(e)) => match e.kind() {
                ErrorKind::AlreadyExists => -2,
                _ => std::i64::MIN,
            },
        })
    }
}
