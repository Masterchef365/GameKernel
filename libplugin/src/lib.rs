mod debug;
mod maybe;
mod reactor;
mod task_pool;
pub use debug::debug;
pub use socket::{Socket, SocketListener};
pub use task_pool::spawn;

pub use futures::io::{AsyncReadExt, AsyncWriteExt};
pub use futures::stream::StreamExt;

#[cfg(target_arch = "wasm32")]
pub mod wasm_socket;
#[cfg(target_arch = "wasm32")]
pub use wasm_socket as socket;

#[cfg(not(target_arch = "wasm32"))]
pub mod native_socket;
#[cfg(not(target_arch = "wasm32"))]
pub use native_socket as socket;

use std::pin::Pin;
use std::future::Future;
use std::task::{Context, Poll};
#[inline]
pub async fn yield_now() {
    YieldNow(false).await
}
struct YieldNow(bool);
impl Future for YieldNow {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.0 {
            self.0 = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}
