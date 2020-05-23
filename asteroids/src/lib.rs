use libplugin::{debug, spawn, Socket};

#[no_mangle]
pub extern "C" fn main() {
    debug("Asteroids init!");
    std::panic::set_hook(Box::new(|info| {
        debug(&info.to_string());
    }));
    spawn(asteroids());
}

async fn asteroids() {
    debug("Client connecting...");
    let socket = Socket::connect("renderer", 0).unwrap().await.unwrap();
    debug("Client connected!");

    let mut conn = render::RendererConn::new(socket);

    let id = conn
        .add_object(render::ObjectData {
            data: Box::new([(
                render::Point2::origin(),
                render::Point2::new(1.0, 1.0),
                render::Point3::new(1.0, 0.0, 1.0),
            )]),
            transform: render::Translation2::identity(),
        })
        .await;

    let mut x = 0.0;
    let mut y = 0.0;
    let rate = 1.0;
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
        conn.set_transform(id, render::Translation2::new(x, y))
            .await;
    }
}
