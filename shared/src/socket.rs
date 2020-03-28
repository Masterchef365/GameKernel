use crate::reactor;
use crate::Handle;
use futures::future::{self, Future};
use futures::io::{AsyncRead, AsyncWrite};
use futures::Stream;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Either represents an error, or a 4 byte integer
type Maybe = i64;

extern "C" {
    fn connect(handle: Handle, peer: *const u8, len: usize) -> Maybe;
    fn listen(handle: Handle) -> Maybe;
    fn handle() -> Handle;
    fn close(handle: Handle); // -> Maybe?

    fn read(handle: Handle, buffer: *mut u8, len: usize) -> Maybe;
    fn write(handle: Handle, buffer: *const u8, len: usize) -> Maybe;
}

type PolledIO<T> = Poll<Result<T, io::Error>>;

fn poll_ffi(retval: Maybe, handle: Handle, cx: &mut Context) -> PolledIO<u32> {
    match retval {
        r if r >= 0 => Poll::Ready(Ok(r as u32)),
        -1 => {
            reactor::register(handle, cx.waker().clone());
            Poll::Pending
        }
        _ => Poll::Ready(Err(io::Error::new(
            io::ErrorKind::Other,
            "Unhandled error type",
        ))),
    }
}

pub struct Socket {
    handle: Handle,
}

impl Socket {
    pub fn connect<'a>(peer: &'a str) -> impl Future<Output = io::Result<Self>> + 'a {
        let handle = unsafe { handle() };
        future::poll_fn(move |cx| {
            let ret = unsafe { connect(handle, peer.as_ptr(), peer.len()) };
            let poll = poll_ffi(ret, handle, cx);
            if poll.is_pending() {
                reactor::register(handle, cx.waker().clone());
            }
            poll.map(|result| {
                result.map(|handle| Self {
                    handle: handle as Handle,
                })
            })
        })
    }

    pub(crate) fn from_handle(handle: Handle) -> Self {
        Self { handle }
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        unsafe { close(self.handle) }
    }
}

impl AsyncWrite for Socket {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> PolledIO<usize> {
        let ret = unsafe { write(self.handle, buf.as_ptr(), buf.len()) };
        poll_ffi(ret, self.handle, cx).map(|v| v.map(|v| v as usize))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> PolledIO<()> {
        todo!("Some sort of flush() here")
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> PolledIO<()> {
        todo!("Call close() here")
    }
}

impl AsyncRead for Socket {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context, buf: &mut [u8]) -> PolledIO<usize> {
        let ret = unsafe { read(self.handle, buf.as_mut_ptr(), buf.len()) };
        poll_ffi(ret, self.handle, cx).map(|v| v.map(|v| v as usize))
    }
}

struct SocketListener {
    handle: Handle,
}

impl SocketListener {
    pub fn new() -> Self {
        Self {
            handle: unsafe { handle() },
        }
    }
}

impl Stream for SocketListener {
    type Item = Socket;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let ret = unsafe { listen(self.handle) };
        poll_ffi(ret, self.handle, cx).map(|v| v.ok().map(Socket::from_handle))
    }
}
