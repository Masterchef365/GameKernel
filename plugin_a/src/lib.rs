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
    let mut n = 0;
    loop {
        let mut buf = [0; 512];
        let nb = socket.read(&mut buf).await.unwrap();
        debug(&String::from_utf8(buf[..nb].to_vec()).unwrap());
        socket.write(format!("Message from server! {}", n).as_bytes()).await.unwrap();
        n += 1;
    }
}
