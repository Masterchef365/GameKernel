//use futures::io::{AsyncReadExt, AsyncWriteExt};
use futures::StreamExt;
use futures::SinkExt;
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use libplugin::debug;
use libplugin::{spawn, Socket, SocketListener};

#[no_mangle]
pub extern "C" fn main() {
    debug("Server init!");
    spawn(server());
}

async fn server() {
    debug("Server started");
    let mut listener = SocketListener::new(5062).unwrap();
    while let Some(Ok(connection)) = listener.next().await {
        debug("Server got new connection");
        spawn(handle_connection(connection));
    }
}

async fn handle_connection(socket: Socket) {
    let mut framed = Framed::new(socket.compat(), LengthDelimitedCodec::new());
    debug("Server handling new connection");
    loop {
        let bytes = framed.next().await.unwrap().unwrap();
        debug(&String::from_utf8(bytes.to_vec()).unwrap());
        framed.send(bytes.into()).await.unwrap();
    }
}
