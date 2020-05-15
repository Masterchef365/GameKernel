use kiss3d::window::Window;
use libplugin::{spawn, yield_now, AsyncReadExt, Socket, SocketListener, StreamExt};
use nalgebra::Point3;
use std::cell::RefCell;
use std::rc::Rc;

#[no_mangle]
pub extern "C" fn main() {
    println!("Renderer loaded");
    spawn(server())
}

type PointCollection = Rc<RefCell<Vec<Point3<f32>>>>;

async fn server() {
    let mut listener = SocketListener::new(0).unwrap();
    let points = PointCollection::default();
    points.borrow_mut().push(Point3::new(0.0, 0.0, 0.0));
    spawn(render_loop(points.clone()));
    while let Some(Ok(connection)) = listener.next().await {
        spawn(handle_connection(connection, points.clone()));
    }
}

async fn render_loop(points: PointCollection) {
    let mut window = Window::new("Game kernel :: OwO");
    while window.render() {
        for point in points.borrow().iter() {
            window.draw_point(point, &Point3::new(1.0, 1.0, 1.0));
        }
        yield_now().await;
    }
}

async fn handle_connection(mut s: Socket, points: PointCollection) {
    let mut buf = [0u8; 20];
    println!("Renderer Handling connection");
    loop {
        s.read_exact(&mut buf).await.unwrap();
        println!("BUFFER");
        let msg: Point3<f32> = bincode::deserialize(&buf).unwrap();
        points.borrow_mut().push(msg);
    }
}
