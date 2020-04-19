use crate::reactor;
use crate::maybe::{Handle, Maybe};
use futures::future::{self, Future};
use futures::io::{AsyncRead, AsyncWrite};
use futures::Stream;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

#[cfg(target_arch="wasm32")]
extern "C" {
    fn connect(peer: *const u8, len: usize, port: u16) -> Maybe;
    fn listener_create(port: u16) -> Maybe;
    fn listen(handle: Handle) -> Maybe;
    fn close(handle: Handle);

    fn read(handle: Handle, buffer: *mut u8, len: usize) -> Maybe;
    fn write(handle: Handle, buffer: *const u8, len: usize) -> Maybe;
}

#[cfg(not(target_arch="wasm32"))]
mod native {
    use super::*;
    pub unsafe fn connect(peer: *const u8, len: usize, port: u16) -> Maybe { todo!() }
    pub unsafe fn listener_create(port: u16) -> Maybe { todo!() }
    pub unsafe fn listen(handle: Handle) -> Maybe { todo!() }
    pub unsafe fn close(handle: Handle) { todo!() }

    pub unsafe fn read(handle: Handle, buffer: *mut u8, len: usize) -> Maybe { todo!() }
    pub unsafe fn write(handle: Handle, buffer: *const u8, len: usize) -> Maybe { todo!() }
}

#[cfg(not(target_arch="wasm32"))]
use native::*;

fn poll_ffi(retval: Maybe, handle: Handle, cx: &Context) -> Poll<io::Result<u32>> {
    let poll = retval.into_poll();
    if poll.is_pending() {
        reactor::register(handle, cx.waker().clone());
    }
    poll
}

pub struct Socket {
    handle: Handle,
}

impl Socket {
    pub fn connect<'a>(
        peer: &'a str,
        port: u16,
    ) -> io::Result<impl Future<Output = io::Result<Self>> + 'a> {
        let handle = unsafe { connect(peer.as_ptr(), peer.len(), port) }
            .errorkind()
            .map_err(io::Error::from)?;

        Ok(future::poll_fn(move |cx| {
            let poll = poll_ffi(unsafe { listen(handle) }, handle, cx);
            if poll.is_ready() {
                unsafe { close(handle); }
            }
            poll.map(|result| {
                result.map(|handle| Self {
                    handle: handle as Handle,
                })
            })
        }))
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        unsafe { close(self.handle) }
    }
}

impl AsyncWrite for Socket {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        let ret = unsafe { write(self.handle, buf.as_ptr(), buf.len()) };
        poll_ffi(ret, self.handle, cx).map(|v| v.map(|v| v as usize))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
        todo!("Some sort of flush() here")
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
        todo!("Call close() here")
    }
}

impl AsyncRead for Socket {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let ret = unsafe { read(self.handle, buf.as_mut_ptr(), buf.len()) };
        poll_ffi(ret, self.handle, cx).map(|v| v.map(|v| v as usize))
    }
}

pub struct SocketListener {
    handle: Handle,
}

impl Drop for SocketListener {
    fn drop(&mut self) {
        unsafe { close(self.handle) }
    }
}

impl SocketListener {
    pub fn new(port: u16) -> io::Result<Self> {
        unsafe { listener_create(port) }
            .errorkind()
            .map(|handle| Self { handle })
            .map_err(io::Error::from)
    }
}

impl Stream for SocketListener {
    type Item = Socket;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let ret = unsafe { listen(self.handle) };
        poll_ffi(ret, self.handle, cx).map(|v| v.ok().map(|handle| Socket { handle }))
    }
}
