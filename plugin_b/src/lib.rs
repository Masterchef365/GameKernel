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
    let mut socket = Socket::connect("librender", 0).unwrap().await.unwrap();
    debug("Connected!...");
    let len = 10usize;
    for x in 0..len {
        for y in 0..len {
            for z in 0..len {
                let buf = serialize(&Point3::new(
                    x as f32 / len as f32,
                    y as f32 / len as f32,
                    z as f32 / len as f32,
                ))
                .unwrap();
                socket.write(&buf).await.unwrap();
                yield_now().await;
            }
        }
    }
    debug("Client done");
}
