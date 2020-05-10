use bincode::serialize;
use futures::SinkExt;
use futures::StreamExt;
use libplugin::{debug, spawn, yield_now, AsyncReadExt, AsyncWriteExt, Socket};
use nalgebra::Point3;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tokio_util::compat::FuturesAsyncReadCompatExt;

#[no_mangle]
pub extern "C" fn main() {
    debug("Client init!");
    spawn(connect());
}

async fn connect() {
    debug("Client connecting...");
    let socket = Socket::connect("plugin_a", 5062).unwrap().await.unwrap();
    let mut framed = Framed::new(socket.compat(), LengthDelimitedCodec::new());
    debug("Client Connected!...");
    let mut n = 0;
    loop {
        framed
            .send(format!("Message from client! {}", n).into())
            .await
            .unwrap();
        let bytes = framed.next().await.unwrap().unwrap();
        if n % 1000 == 0 {
            debug(&String::from_utf8(bytes.to_vec()).unwrap());
        }
        n += 1;
    }
}
