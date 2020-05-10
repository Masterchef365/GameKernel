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
    let mut buf = [0; 512];
    loop {
        debug("Client Connected!...");
        debug(&format!("{:?}", socket.write(b"Message from client!").await));
        debug("Client done");
        debug(&format!("{:?}", socket.read(&mut buf).await));
        debug(&String::from_utf8(buf.to_vec()).unwrap());
    }
}
