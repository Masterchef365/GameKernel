use futures::io::{AsyncReadExt, AsyncWriteExt};
use shared::{spawn, Socket};
use shared::debug;

#[no_mangle]
pub extern "C" fn main() {
    debug("FRICK");
    spawn(test("Bitchass"));
    /*
    spawn(test("FUck nut"));
    */
}

async fn test(message: &str) {
    let mut socket = Socket::connect("ec_database", 0).unwrap().await.unwrap();
    socket.write(message.as_bytes()).await.unwrap();
    let mut bytes2 = [0u8; 9];
    socket.read(&mut bytes2).await.unwrap();
    socket.write(&bytes2).await.unwrap();
}
