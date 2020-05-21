use futures::executor::LocalPool;
use futures::task::SpawnExt;
use loopback::Loopback;
use nalgebra::{Point3, Translation3};
use render::*;

fn main() {
    let mut pool = LocalPool::new();
    let spawner = pool.spawner();
    let (a, b) = Loopback::pair();
    let renderer = Renderer::new("UwU".into());
    spawner
        .spawn(Renderer::handle_client(renderer.clone(), b))
        .unwrap();
    spawner
        .spawn(async move {
            let mut conn = RendererObjects::new(a);
            let id = conn.add_object(ObjectData {
                data: Box::new([(
                    Point3::origin(),
                    Point3::new(1.0, 1.0, 1.0),
                    Point3::new(1.0, 1.0, 1.0),
                )]),
                transform: Translation3::identity(),
            })
            .await;
            let mut i: f32 = 0.0;
            loop {
                conn.set_transform(id, Translation3::new(i.cos(), 0.0, 0.0)).await;
                i += 0.0001;
            }
        })
        .unwrap();
    pool.run();
    std::thread::park();
}
