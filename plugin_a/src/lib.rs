use futures::io::{AsyncReadExt, AsyncWriteExt};
use futures::stream::StreamExt;
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

async fn handle_connection(mut socket: Socket) {
    debug("Server handling new connection");
    loop {
        let mut buf = [0; 512];
        socket.read(&mut buf).await.unwrap();
        debug("SERVER GOT MESSAGE:");
        debug(&String::from_utf8(buf.to_vec()).unwrap());
        socket.write(b"FUCK MONEY GET BITCHES").await.unwrap();
    }
    /*
    socket.write("Test".as_bytes()).await.unwrap();
    let mut bytes2 = [0u8; 9];
    socket.read(&mut bytes2).await.unwrap();
    socket.write(&bytes2).await.unwrap();
    */
}
