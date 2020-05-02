use futures::executor::LocalPool;
use futures::task::LocalSpawnExt;
use once_cell::unsync::Lazy;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub static mut TASK_POOL: Lazy<LocalPool> = Lazy::new(LocalPool::new);

#[no_mangle]
pub unsafe extern "C" fn run_tasks() {
    TASK_POOL.run_until_stalled();
}

pub fn spawn<F: Future<Output = ()> + 'static>(f: F) {
    unsafe {
        TASK_POOL.spawner().spawn_local(f).unwrap();
    }
}

// Credit: async-std authors
#[inline]
pub async fn yield_now() {
    YieldNow(false).await
}

struct YieldNow(bool);
impl Future for YieldNow {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.0 {
            Poll::Ready(())
        } else {
            self.0 = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}
