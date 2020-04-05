use futures::task::LocalSpawnExt;
use futures::executor::LocalPool;
use std::future::Future;
use once_cell::unsync::Lazy;

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

