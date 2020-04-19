use crate::maybe::{Handle, Maybe};
use crate::reactor;
use futures::future::{self, Future};
use futures::io::{AsyncRead, AsyncWrite};
use futures::Stream;
use std::cell::Cell;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use game_kernel::socket::SocketManager;
static mut SOCKET_MANAGER: Option<&'static mut SocketManager> = None;
#[no_mangle]

pub unsafe extern "C" fn set_socketmanager(sm: &'static mut SocketManager) {
    SOCKET_MANAGER = Some(sm);
}

fn sm() -> &'static mut SocketManager {
    unsafe { SOCKET_MANAGER.as_mut().expect("Socket manager not set") }
}

pub struct Socket {
    handle: Handle,
}

pub struct SocketListener {
    handle: Handle,
}

impl Socket {
    pub fn connect<'a>(
        peer: &'a str,
        port: u16,
    ) -> io::Result<impl Future<Output = io::Result<Self>> + 'a> {
        let listener = match sm().connect(peer, port) {
            Poll::Ready(Ok(l)) => Ok(l),
            Poll::Pending => Err(io::Error::new(io::ErrorKind::WouldBlock, "Rejected")),
            Poll::Ready(e) => e,
        }?;

        Ok(future::poll_fn(move |cx| match sm().listen(listener) {
            Poll::Ready(Ok(handle)) => {
                sm().close(listener);
                Poll::Ready(Ok(Self { handle }))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => {
                reactor::register(listener, cx.waker().clone());
                Poll::Pending
            }
        }))
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        sm().close(self.handle)
    }
}

impl AsyncWrite for Socket {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        unsafe {
            let buf = std::mem::transmute::<_, &[Cell<u8>]>(buf);
            let poll = sm().write(self.handle, buf);
            if poll.is_pending() {
                reactor::register(self.handle, cx.waker().clone());
            }
            poll.map(|v| v.map(|v| v as usize))
        }
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
        unsafe {
            let buf = std::mem::transmute::<_, &mut [Cell<u8>]>(buf);
            let poll = sm().read(self.handle, buf);
            if poll.is_pending() {
                reactor::register(self.handle, cx.waker().clone());
            }
            poll.map(|v| v.map(|v| v as usize))
        }
    }
}

impl Drop for SocketListener {
    fn drop(&mut self) {
        sm().close(self.handle)
    }
}

impl SocketListener {
    pub fn new(port: u16) -> io::Result<Self> {
        match sm().listener_create(port) {
            Poll::Ready(Ok(handle)) => Ok(Self { handle }),
            Poll::Pending => Err(io::Error::new(io::ErrorKind::WouldBlock, "Rejected")),
            Poll::Ready(Err(e)) => Err(e),
        }
    }
}

impl Stream for SocketListener {
    type Item = Socket;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let poll = sm().listen(self.handle);
        if poll.is_pending() {
            reactor::register(self.handle, cx.waker().clone());
        }
        poll.map(|h| h.ok().map(|handle| Socket { handle }))
    }
}
