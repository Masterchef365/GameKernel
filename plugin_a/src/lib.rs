use futures::io::{AsyncReadExt, AsyncWriteExt};
use futures::stream::StreamExt;
use libplugin::{spawn, SocketListener, Socket};
use libplugin::debug;

#[no_mangle]
pub extern "C" fn main() {
    debug("Server init!");
    spawn(server());
}

async fn server() {
    debug("Server started");
    let mut listener = SocketListener::new(5062).unwrap();
    while let Some(connection) = listener.next().await {
        debug("Server got new connection");
        spawn(handle_connection(connection));
    }
}

async fn handle_connection(socket: Socket) {
    debug("Server handling new connection");
    futures::pending!();
    /*
    socket.write("Test".as_bytes()).await.unwrap();
    let mut bytes2 = [0u8; 9];
    socket.read(&mut bytes2).await.unwrap();
    socket.write(&bytes2).await.unwrap();
    */
}
