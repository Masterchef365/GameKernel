use futures::future::{poll_fn, Future};
use futures::io::AsyncWrite;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

extern "C" {
    fn socket(name: *const u8, len: usize) -> u32;
    fn write(fd: u32, buf: *const u8, len: usize) -> i64;
    fn read(fd: u32, buf: *const u8, len: usize) -> i64;
}

fn retvalue(retval: i64) -> Poll<Result<usize, io::Error>> {
    match retval {
        r if r >= 0 => Poll::Ready(Ok(r as usize)),
        -1 => Poll::Pending,
        r => Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, "Bitch"))),
    }
}

pub struct Socket {
    fd: u32,
}

impl Socket {
    pub fn new() -> Self {
        Self { fd: 0 }
    }
}

impl AsyncWrite for Socket {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let ret = unsafe { write(0, buf.as_ptr(), buf.len()) };
        if ret >= 0 {
            Poll::Ready(Ok(ret as usize))
        } else {
            Poll::Pending
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }
}
