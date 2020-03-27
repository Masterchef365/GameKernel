use futures::io::{AsyncReadExt, AsyncWriteExt};
use futures::task::LocalSpawnExt;
use futures::task::SpawnExt;
use shared::Socket;

use futures::executor::LocalPool;
use std::future::Future;
use once_cell::unsync::Lazy;

static mut TASK_POOL: Lazy<LocalPool> = Lazy::new(LocalPool::new);

fn spawn<F: Future<Output = ()> + 'static>(f: F) {
    unsafe {
        TASK_POOL.spawner().spawn_local(f).unwrap();
    }
}

#[no_mangle]
pub unsafe extern "C" fn poll() {
    TASK_POOL.run_until_stalled();
}

#[no_mangle]
pub unsafe extern "C" fn main() {
    spawn(test("Bitchass"));
    spawn(test("FUck nut"));
}

async fn test(message: &str) {
    let mut socket = Socket::new("ec_database");
    let mut socket = Socket::new("ec_database");
    socket.write(message.as_bytes()).await.unwrap();
    let mut bytes2 = [0u8; 9];
    socket.read(&mut bytes2).await.unwrap();
    socket.write(&bytes2).await.unwrap();
    //write(write(bytes).await.to_string().as_bytes()).await;
}
