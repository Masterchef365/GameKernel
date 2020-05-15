//use futures::io::{AsyncReadExt, AsyncWriteExt};
use futures::SinkExt;
use futures::StreamExt;
use libplugin::debug;
use libplugin::{spawn, Socket, SocketListener};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tokio_util::compat::FuturesAsyncReadCompatExt;

#[no_mangle]
pub extern "C" fn main() {
    debug("Server init!");
    std::panic::set_hook(Box::new(|info| {
        debug(&format!("{}", info));
    }));
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
