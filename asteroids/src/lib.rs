mod objects;
use futures::{AsyncRead, AsyncWrite};
use libplugin::{debug, spawn, Socket};
use objects::*;
use render::{Id, Isometry2, Point2, Point3, RendererConn, Vector2};

#[derive(Copy, Clone)]
pub struct Box2D {
    pub min: Point2<f32>,
    pub max: Point2<f32>,
}

fn wrap(min: f32, v: f32, max: f32) -> f32 {
    if v < min {
        max
    } else if v > max {
        min
    } else {
        v
    }
}

impl Box2D {
    pub fn new(min: Point2<f32>, max: Point2<f32>) -> Self {
        assert!(min.x <= max.x);
        assert!(min.y <= max.y);
        Self { min, max }
    }

    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    pub fn wrap(&self, pt: Point2<f32>) -> Point2<f32> {
        let x = wrap(self.min.x, pt.x, self.max.x);
        let y = wrap(self.min.y, pt.y, self.max.y);
        Point2::new(x, y)
    }
}

#[no_mangle]
pub extern "C" fn main() {
    debug("Asteroids init!");
    std::panic::set_hook(Box::new(|info| {
        debug(&info.to_string());
    }));
    spawn(asteroids());
}

struct Ship {
    pub position: Point2<f32>,
    pub velocity: Vector2<f32>,
    pub angle: f32,
}

impl Ship {
    pub fn new() -> Self {
        Self {
            position: Point2::origin(),
            velocity: Vector2::new(0.0, 0.0),
            angle: 0.0,
        }
    }

    pub fn update(&mut self, screen: Box2D, fire_engine: bool, left_key: bool, right_key: bool) {
        const ROT_RATE: f32 = 0.08;
        if left_key {
            self.angle += ROT_RATE;
        }
        if right_key {
            self.angle -= ROT_RATE;
        }

        const ACCEL: f32 = 0.1;
        let acceleration = if fire_engine {
            Vector2::new(ACCEL * self.angle.cos(), ACCEL * self.angle.sin())
        } else {
            Vector2::new(0.0, 0.0)
        };

        self.velocity += acceleration;
        self.position += self.velocity;
        self.position = screen.wrap(self.position);
    }

    pub fn isometry(&self) -> Isometry2<f32> {
        Isometry2::new(self.position.coords, self.angle)
    }
}

async fn asteroids() {
    debug("Client connecting...");
    let socket = Socket::connect("renderer", 0).unwrap().await.unwrap();
    debug("Client connected!");

    let mut conn = RendererConn::new(socket);

    for x in (-300..300).step_by(50) {
        for y in (-300..300).step_by(50) {}
    }

    let mut ship = Ship::new();
    let ship_rid = conn
        .add_object(render::ObjectData {
            data: rocket(),
            transform: Isometry2::identity(),
        })
        .await;

    let screen = Box2D::new(Point2::new(-360.0, -360.0), Point2::new(360.0, 360.0));

    let _ = conn
        .add_object(render::ObjectData {
            data: rectangle(&screen, Point3::new(1.0, 1.0, 1.0)),
            transform: Isometry2::identity(),
        })
        .await;

    loop {
        let info = conn.wait_frame().await;
        ship.update(
            screen,
            info.keys.contains(&' '),
            info.keys.contains(&'A'),
            info.keys.contains(&'D'),
        );
        conn.set_transform(ship_rid, ship.isometry()).await;
    }
}
