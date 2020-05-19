use futures::SinkExt;
use futures::StreamExt;
use libplugin::{debug, spawn, yield_now, AsyncReadExt, AsyncWriteExt, Socket};

#[no_mangle]
pub extern "C" fn main() {
    debug("Client init!");
    spawn(connect());
}

async fn connect() {
    debug("Client connecting...");
    let socket = Socket::connect("renderer", 0).unwrap().await.unwrap();
    debug("Client connected!");

    let mut conn = render::RendererConnection::new(socket);
    let id = conn
        .add_object(render::ObjectData {
            data: Box::new([(
                render::Point3::origin(),
                render::Point3::new(1.0, 1.0, 1.0),
                render::Point3::new(1.0, 1.0, 1.0),
            )]),
            transform: render::Translation3::identity(),
        })
        .await;
    let mut i: f32 = 0.0;
    loop {
        conn.set_transform(id, render::Translation3::new(i.cos(), 0.0, 0.0))
            .await;
        i += 0.00001;
    }
}
