mod objects;
use futures::{AsyncRead, AsyncWrite};
use libplugin::{debug, spawn, Socket};
use render::{Id, Isometry2, Point2, Point3, RendererConn, Vector2};
use std::any::Any;
use std::collections::HashMap;

type EntityId = u32;
type ComponentId = u32;

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
    pub render_id: Id,
}

impl Ship {
    pub async fn new<S: AsyncRead + AsyncWrite + Unpin>(conn: &mut RendererConn<S>) -> Self {
        let render_id = conn
            .add_object(render::ObjectData {
                data: objects::rocket(),
                transform: Isometry2::identity(),
            })
            .await;
        Self {
            render_id,
            position: Point2::origin(),
            velocity: Vector2::new(0.0, 0.0),
            angle: 0.0,
        }
    }

    pub async fn update<S: AsyncRead + AsyncWrite + Unpin>(
        &mut self,
        conn: &mut RendererConn<S>,
        screen: Box2D,
        fire_engine: bool,
        left_key: bool,
        right_key: bool,
    ) {
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
        conn.set_transform(
            self.render_id,
            Isometry2::new(self.position.coords, self.angle),
        )
        .await;
    }
}

struct Bullet {
    pub position: Point2<f32>,
    pub velocity: Vector2<f32>,
    pub life: u32,
    pub render_id: Id,
}

impl Bullet {
    pub async fn new<S: AsyncRead + AsyncWrite + Unpin>(
        conn: &mut RendererConn<S>,
        position: Point2<f32>,
        velocity: Vector2<f32>,
    ) -> Self {
        let render_id = conn
            .add_object(render::ObjectData {
                data: objects::bullet(),
                transform: Isometry2::new(position.coords, 0.0),
            })
            .await;
        Self {
            render_id,
            position,
            velocity,
            life: 90,
        }
    }

    pub async fn update<S: AsyncRead + AsyncWrite + Unpin>(
        &mut self,
        conn: &mut RendererConn<S>,
        screen: Box2D,
    ) {
        self.position += self.velocity;
        self.position = screen.wrap(self.position);
        conn.set_transform(self.render_id, Isometry2::new(self.position.coords, 0.0))
            .await;
        self.life -= 1;
    }
}

async fn asteroids() {
    debug("Client connecting...");
    let socket = Socket::connect("renderer", 0).unwrap().await.unwrap();
    debug("Client connected!");

    let mut conn = RendererConn::new(socket);

    let screen = Box2D::new(Point2::new(-360.0, -360.0), Point2::new(360.0, 360.0));
    let _screen_rect = conn
        .add_object(render::ObjectData {
            data: objects::rectangle(&screen, Point3::new(1.0, 1.0, 1.0)),
            transform: Isometry2::identity(),
        })
        .await;

    let mut ship = Ship::new(&mut conn).await;
    let mut bullets: Vec<Bullet> = Vec::new();

    loop {
        let info = conn.wait_frame().await;
        ship.update(
            &mut conn,
            screen,
            info.keys.contains(&'W'),
            info.keys.contains(&'A'),
            info.keys.contains(&'D'),
        )
        .await;
        if info.keys.contains(&' ') {
            bullets.push(
                Bullet::new(
                    &mut conn,
                    ship.position,
                    ship.velocity + Vector2::new(ship.angle.cos(), ship.angle.sin()) * 8.5,
                )
                .await,
            );
        }
        for bullet in &mut bullets {
            bullet.update(&mut conn, screen).await;
            if bullet.life == 0 {
                conn.delete_object(bullet.render_id).await;
            }
        }
        bullets.retain(|b| b.life > 0);
    }
}
