use futures::io::{AsyncReadExt, AsyncWriteExt};
use libplugin::{spawn, Socket};
use libplugin::debug;

#[no_mangle]
pub extern "C" fn main() {
    debug("Client init!");
    spawn(connect());
}

async fn connect() {
    debug("Client connecting...");
    let mut socket = Socket::connect("plugin_a", 5062).unwrap().await.unwrap();
    debug("Client connected!");
}
