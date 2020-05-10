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
    let mut buf = [0; 512];
    loop {
        socket.write(b"Message from client!").await.unwrap();
        let n = socket.read(&mut buf).await.unwrap();
        debug(&String::from_utf8(buf[..n].to_vec()).unwrap());
    }
}
