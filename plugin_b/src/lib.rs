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
    loop {
        framed
            .send("Message from client!".into())
            .await
            .unwrap();
        let bytes = framed.next().await.unwrap().unwrap();
        debug(&String::from_utf8(bytes.to_vec()).unwrap());
    }
}
