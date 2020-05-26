mod objects;
use futures::{AsyncRead, AsyncWrite};
use libplugin::{debug, spawn, Socket};
use rand::distributions::{Distribution, Uniform};
use rand::rngs::SmallRng;
use rand::SeedableRng;
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
    pub duration: u32,
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
            duration: 90,
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
        self.duration -= 1;
    }
}

struct Asteroid {
    pub position: Point2<f32>,
    pub velocity: Vector2<f32>,
    pub render_id: Id,
    pub radius: f32,
    pub life: u32,
}

impl Asteroid {
    pub async fn new<S: AsyncRead + AsyncWrite + Unpin>(
        conn: &mut RendererConn<S>,
        position: Point2<f32>,
        velocity: Vector2<f32>,
        radius: f32,
    ) -> Self {
        let render_id = conn
            .add_object(render::ObjectData {
                data: objects::asteroid(radius),
                transform: Isometry2::new(position.coords, 0.0),
            })
            .await;
        Self {
            radius,
            render_id,
            position,
            velocity,
            life: 5,
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
    }

    pub fn collides_with(&self, object: Point2<f32>) -> bool {
        (self.position - object).magnitude() <= self.radius
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
    let mut bullets = Vec::new();
    let mut asteroids = Vec::new();

    let mut rng = SmallRng::seed_from_u64(3489277);
    let pos = Uniform::new(-300.9, 300.0);
    let size = Uniform::new(10.0, 60.0);
    let vel = Uniform::new(0.1, 1.0);

    for _ in 0..10 {
        asteroids.push(
            Asteroid::new(
                &mut conn,
                Point2::new(pos.sample(&mut rng), pos.sample(&mut rng)),
                Vector2::new(vel.sample(&mut rng), vel.sample(&mut rng)),
                size.sample(&mut rng),
            )
            .await,
        );
    }

    'mainloop: loop {
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
            if bullet.duration == 0 {
                conn.delete_object(bullet.render_id).await;
            }
        }
        let mut new_roids = Vec::new();
        for asteroid in &mut asteroids {
            asteroid.update(&mut conn, screen).await;
            if asteroid.collides_with(ship.position) {
                debug("You died!");
                break 'mainloop;
            }
            for bullet in &mut bullets {
                if asteroid.collides_with(bullet.position) {
                    bullet.duration = 0;
                    conn.delete_object(bullet.render_id).await;
                    asteroid.life -= 1;
                }
            }
            if asteroid.life == 0 {
                conn.delete_object(asteroid.render_id).await;
                if asteroid.radius > 10.0 {
                    new_roids.push(
                        Asteroid::new(
                            &mut conn,
                            asteroid.position,
                            asteroid.velocity / 2.0,
                            asteroid.radius / 2.0,
                        )
                        .await,
                    );
                    new_roids.push(
                        Asteroid::new(
                            &mut conn,
                            asteroid.position,
                            asteroid.velocity / -2.0,
                            asteroid.radius / 2.0,
                        )
                        .await,
                    );
                }
            }
        }
        bullets.retain(|b| b.duration > 0);
        asteroids.retain(|a| a.life > 0);
        asteroids.extend(new_roids.drain(..));
        if asteroids.is_empty() {
            debug("You won!");
            break 'mainloop;
        }
    }
}
