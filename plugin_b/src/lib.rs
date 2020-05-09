use bincode::serialize;
use libplugin::{debug, spawn, yield_now, AsyncReadExt, AsyncWriteExt, Socket};
use nalgebra::Point3;

#[no_mangle]
pub extern "C" fn main() {
    debug("Client init!");
    spawn(connect());
}

async fn connect() {
    debug("Client connecting...");
    let mut socket = Socket::connect("plugin_a", 5062).unwrap().await.unwrap();
    debug("Client Connected!...");
    socket.write(b"Message from client!");
    debug("Client done");
}
