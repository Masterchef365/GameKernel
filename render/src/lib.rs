use kiss3d::window::Window;
use libplugin::{spawn, AsyncReadExt, AsyncWriteExt, Socket, SocketListener, StreamExt, yield_now};
use nalgebra::Point3;
use std::cell::RefCell;
use std::rc::Rc;

#[no_mangle]
pub extern "C" fn main() {
    println!("Renderer loaded");
    spawn(server())
}

async fn server() {
    let mut listener = SocketListener::new(0).unwrap();
    let points = Rc::new(RefCell::new(Vec::new()));
    points.borrow_mut().push(Point3::new(0.0, 0.0, 0.0));
    spawn(render_loop(points.clone()));
    while let Some(Ok(connection)) = listener.next().await {
        spawn(handle_connection(connection, points.clone()));
    }
}

async fn render_loop(points: Rc<RefCell<Vec<Point3<f32>>>>) {
    let mut window = Window::new("Game kernel :: OwO");
    while window.render() {
        for point in points.borrow().iter() {
            window.draw_point(point, &Point3::new(1.0, 1.0, 1.0));
        }
        yield_now().await;
    }
}

async fn handle_connection(mut s: Socket, points: Rc<RefCell<Vec<Point3<f32>>>>) {
    let mut buf = [0u8; 20];
    println!("Renderer Handling connection");
    loop {
        s.read(&mut buf).await.unwrap();
        let msg: Point3<f32> = bincode::deserialize(&buf).unwrap();
        points.borrow_mut().push(msg);
    }
}
