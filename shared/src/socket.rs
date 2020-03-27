use futures::io::{AsyncRead, AsyncWrite};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

extern "C" {
    fn socket(name: *const u8, len: usize) -> u32;
    fn write(fd: u32, buf: *const u8, len: usize) -> i64;
    fn read(fd: u32, buf: *mut u8, len: usize) -> i64;
    fn close(fd: u32);
    fn set_wake(fd: u32, interested: bool);
}

type PolledIO<T> = Poll<Result<T, io::Error>>;

fn retvalue(retval: i64) -> PolledIO<usize> {
    match retval {
        r if r >= 0 => Poll::Ready(Ok(r as usize)),
        -1 => Poll::Pending,
        _ => Poll::Ready(Err(io::Error::new(
            io::ErrorKind::Other,
            "Unhandled error type",
        ))),
    }
}

pub struct Socket {
    fd: u32,
}

impl Socket {
    pub fn new(peer: &str) -> Self {
        unsafe {
            Self {
                fd: socket(peer.as_ptr(), peer.len()),
            }
        }
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        unsafe { close(self.fd); }
    }
}

impl AsyncWrite for Socket {
    fn poll_write(self: Pin<&mut Self>, _cx: &mut Context, buf: &[u8]) -> PolledIO<usize> {
        retvalue(unsafe { write(self.fd, buf.as_ptr(), buf.len()) })
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> PolledIO<()> {
        todo!("Some sort of flush() here")
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> PolledIO<()> {
        todo!("Call close() here")
    }
}

impl AsyncRead for Socket {
    fn poll_read(self: Pin<&mut Self>, _cx: &mut Context, buf: &mut [u8]) -> PolledIO<usize> {
        retvalue(unsafe { read(self.fd, buf.as_mut_ptr(), buf.len()) })
    }
}
