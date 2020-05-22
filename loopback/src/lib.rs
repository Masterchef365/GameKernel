use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::io::{AsyncRead, AsyncWrite, Error, Result};
use futures::sink::Sink;
use futures::stream::{Peekable, StreamExt};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

const CHANNEL_CAP: usize = 32;
const MAX_BUFFER_SIZE: usize = 16384; // 16k is probably about the biggest message we want to have.

/// An asynchronous loop back socket
pub struct Loopback {
    tx: Sender<Vec<u8>>,
    tx_buf: Vec<u8>,

    rx: Peekable<Receiver<Vec<u8>>>,
    rx_buf: Vec<u8>,
}

impl Loopback {
    /// Create a pair of connected loopback sockets.
    pub fn pair() -> (Self, Self) {
        let (a_tx, b_rx) = channel(CHANNEL_CAP);
        let (b_tx, a_rx) = channel(CHANNEL_CAP);
        (
            Loopback::with_channels(a_tx, a_rx),
            Loopback::with_channels(b_tx, b_rx),
        )
    }

    /// Returns true if this loopback is ready for a read or a write.
    pub fn has_data(&mut self, cx: &mut Context) -> bool {
        !self.rx_buf.is_empty()
            || Pin::new(&mut self.rx).poll_peek(cx).is_ready()
            || Pin::new(&mut self.tx).poll_ready(cx).is_ready()
    }

    fn with_channels(tx: Sender<Vec<u8>>, rx: Receiver<Vec<u8>>) -> Self {
        Self {
            tx,
            rx: rx.peekable(),
            tx_buf: Vec::new(),
            rx_buf: Vec::new(),
        }
    }
}

fn ncerror<T>(_: T) -> io::Error {
    io::Error::from(io::ErrorKind::NotConnected)
}

impl AsyncWrite for Loopback {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>> {
        self.tx_buf.extend_from_slice(buf);
        if self.tx_buf.len() > MAX_BUFFER_SIZE {
            self.poll_flush(cx)?.map(|_| Ok(0))
        } else {
            Poll::Ready(Ok(buf.len()))
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<()>> {
        if self.tx_buf.is_empty() {
            return Poll::Ready(Ok(()));
        }

        if Pin::new(&mut self.tx)
            .poll_ready(cx)
            .map_err(ncerror)?
            .is_pending()
        {
            return Poll::Pending;
        }

        let buf = std::mem::take(&mut self.tx_buf);
        Pin::new(&mut self.tx).start_send(buf).map_err(ncerror)?;
        Pin::new(&mut self.tx).poll_flush(cx).map_err(ncerror)
    }

    fn poll_close(mut self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<()>> {
        self.tx.close_channel();
        self.rx.get_mut().close();
        Poll::Ready(Ok(()))
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
                Poll::Ready(Some(buf)) => self.rx_buf = buf,
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => {
                    return Poll::Ready(Err(Error::from(io::ErrorKind::NotConnected)))
                }
            }
        }

        let n = buf.len().min(self.rx_buf.len());
        buf[..n].copy_from_slice(&self.rx_buf[..n]);
        self.rx_buf.drain(..n);

        Poll::Ready(Ok(n))
    }
}
