use futures::io::{AsyncReadExt, AsyncWriteExt};
use libplugin::{spawn, Socket};
use libplugin::debug;

#[no_mangle]
pub extern "C" fn main() {
    debug("Client init!");
    spawn(connect());
}

async fn connect() {
    debug("Client connecting...");
    let mut socket = Socket::connect("plugin_a", 5062).unwrap().await.unwrap();
    let mut buf = [0; 512];
    socket.write(b"This is the first message sent!").await.unwrap();
    socket.read(&mut buf).await.unwrap();
    debug("CLIENT GOT MESSAGE:");
    debug(&String::from_utf8(buf.to_vec()).unwrap());
}
