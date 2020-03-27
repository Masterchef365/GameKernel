use futures::executor::LocalPool;
use futures::task::SpawnExt;
use futures::io::AsyncWriteExt;
use std::io;
use std::task::Poll;
use shared::Socket;

#[no_mangle]
pub unsafe extern "C" fn main() {
    let mut pool = LocalPool::new();
    pool.spawner().spawn(test()).expect("Failed to spawn task");
    pool.run();
}

async fn test() {
    let mut socket = Socket::new();
    let bytes = b"Bitchass!";
    socket.write(bytes).await.unwrap();
    //write(write(bytes).await.to_string().as_bytes()).await;
}
