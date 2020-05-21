// TODO: Perf increase by buffering sends!

use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::io::{AsyncRead, AsyncWrite, Error, Result};
use futures::sink::Sink;
use futures::stream::{Peekable, StreamExt};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

const CHANNEL_CAP: usize = 128;

pub type PeekRecv<T> = Peekable<Receiver<T>>;

pub struct Loopback {
    tx: Sender<u8>,
    rx: PeekRecv<u8>,
}

impl Loopback {
    pub fn pair() -> (Self, Self) {
        let (a_tx, b_rx) = channel(CHANNEL_CAP);
        let (b_tx, a_rx) = channel(CHANNEL_CAP);
        (
            Loopback {
                tx: a_tx,
                rx: a_rx.peekable(),
            },
            Loopback {
                tx: b_tx,
                rx: b_rx.peekable(),
            },
        )
    }

    pub fn has_data(&mut self, cx: &mut Context) -> bool {
        Pin::new(&mut self.rx).poll_peek(cx).is_ready()
            || Pin::new(&mut self.tx).poll_ready(cx).is_ready()
    }
}

fn ncerror<T>(_: T) -> io::Error {
    io::Error::from(io::ErrorKind::NotConnected)
}

impl AsyncWrite for Loopback {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>> {
        let mut n = 0;
        for byte in buf.iter() {
            let ready = Pin::new(&mut self.tx)
                .poll_ready(cx)
                .map(|v| v.map_err(ncerror))?;
            if ready.is_pending() {
                break;
            }
            Pin::new(&mut self.tx).start_send(*byte).map_err(ncerror)?;
            n += 1;
        }

        if n > 0 {
            Poll::Ready(Ok(n))
        } else {
            Poll::Pending
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<()>> {
        Pin::new(&mut self.tx).poll_flush(cx).map_err(ncerror)
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<()>> {
        Poll::Ready(Ok(())) //TODO
    }
}

impl AsyncRead for Loopback {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        let mut idx = 0;
        loop {
            match self.rx.poll_next_unpin(cx) {
                Poll::Ready(Some(byte)) => {
                    buf[idx] = byte;
                    idx += 1;
                    if idx >= buf.len() {
                        break Poll::Ready(Ok(idx));
                    }
                }
                Poll::Ready(None) => {
                    break Poll::Ready(Err(Error::from(io::ErrorKind::NotConnected)))
                }
                Poll::Pending => {
                    break if idx == 0 {
                        Poll::Pending
                    } else {
                        Poll::Ready(Ok(idx))
                    };
                }
            }
        }
    }
}
