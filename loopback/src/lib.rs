use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::io::{AsyncRead, AsyncWrite, Error, Result};
use futures::sink::Sink;
use futures::stream::{Peekable, StreamExt};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

const CHANNEL_CAP: usize = 32;

pub struct Loopback {
    tx: Sender<Vec<u8>>,
    tx_buf: Vec<u8>,

    rx: Peekable<Receiver<Vec<u8>>>,
    rx_buf: Vec<u8>,
}

impl Loopback {
    pub fn pair() -> (Self, Self) {
        let (a_tx, b_rx) = channel(CHANNEL_CAP);
        let (b_tx, a_rx) = channel(CHANNEL_CAP);
        (Loopback::new(a_tx, a_rx), Loopback::new(b_tx, b_rx))
    }

    fn new(tx: Sender<Vec<u8>>, rx: Receiver<Vec<u8>>) -> Self {
        Self {
            tx,
            rx: rx.peekable(),
            tx_buf: Vec::new(),
            rx_buf: Vec::new(),
        }
    }

    /// If this loopback has data ready for a read, send it.
    pub fn has_data(&mut self, cx: &mut Context) -> bool {
        !self.rx_buf.is_empty()
            || Pin::new(&mut self.rx).poll_peek(cx).is_ready()
            || Pin::new(&mut self.tx).poll_ready(cx).is_ready()
    }
}

fn ncerror<T>(_: T) -> io::Error {
    io::Error::from(io::ErrorKind::NotConnected)
}

impl AsyncWrite for Loopback {
    fn poll_write(mut self: Pin<&mut Self>, _cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>> {
        self.tx_buf.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<()>> {
        if Pin::new(&mut self.tx)
            .poll_ready(cx)
            .map(|v| v.map_err(ncerror))?
            .is_pending()
        {
            return if self.tx_buf.is_empty() {
                Poll::Ready(Ok(()))
            } else {
                Poll::Pending
            };
        }

        let buf = std::mem::take(&mut self.tx_buf);
        Pin::new(&mut self.tx).start_send(buf).map_err(ncerror)?;

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
        if self.rx_buf.is_empty() {
            match self.rx.poll_next_unpin(cx) {
                Poll::Ready(Some(buf)) => {
                    self.rx_buf = buf;
                }
                Poll::Ready(None) => {
                    return Poll::Ready(Err(Error::from(io::ErrorKind::NotConnected)))
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }

        let mut idx = 0;
        let draining_len = buf.len().min(self.rx_buf.len());
        for byte in self.rx_buf.drain(..draining_len) {
            buf[idx] = byte;
            idx += 1;
        }

        Poll::Ready(Ok(idx))
    }
}
