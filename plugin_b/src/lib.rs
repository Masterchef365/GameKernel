use futures::SinkExt;
use futures::StreamExt;
use libplugin::{debug, spawn, yield_now, AsyncReadExt, AsyncWriteExt, Socket};

#[no_mangle]
pub extern "C" fn main() {
    debug("Client init!");
    spawn(connect());
    std::panic::set_hook(Box::new(|info| {
        debug(&format!("{}", info));
    }));
}

async fn connect() {
    debug("Client connecting...");
    let socket = Socket::connect("renderer", 0).unwrap().await.unwrap();
    debug("Client connected!");

    let mut conn = render::RendererConn::new(socket);

    let id = conn
        .add_object(render::ObjectData {
            data: Box::new([(
                render::Point3::origin(),
                render::Point3::new(1.0, 1.0, 1.0),
                render::Point3::new(1.0, 1.0, 0.0),
            )]),
            transform: render::Translation3::identity(),
        })
        .await;

    let mut i: f32 = 0.0;
    let mut x = 0.0;
    let mut y = 0.0;
    let rate = 0.05;
    loop {
        let info = conn.wait_frame().await;
        for key in info.keys {
            match key {
                'W' => y += rate,
                'S' => y -= rate,
                'A' => x -= rate,
                'D' => x += rate,
                _ => (),
            }
        }
        conn.set_transform(id, render::Translation3::new(x, y, 0.0))
            .await;
        i += 0.1;
    }
}
