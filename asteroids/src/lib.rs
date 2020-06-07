mod objects;
use futures::{AsyncRead, AsyncWrite};
use libplugin::{debug, spawn, Socket};
use rand::distributions::{Distribution, Uniform};
use rand::rngs::SmallRng;
use rand::SeedableRng;
use render::{Id, Isometry2, ObjectData, Point2, Point3, RendererConn, Vector2, Rotation2};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

type EntityId = u32;

#[derive(Default)]
struct ECData {
    map: HashMap<EntityId, HashMap<TypeId, Box<dyn Any>>>,
    next_id: EntityId,
}

impl ECData {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get_component<T: Any>(&self, entity: &EntityId) -> Option<&T> {
        self.map.get(entity).and_then(|components| {
            components
                .get(&TypeId::of::<T>())
                .and_then(|v| v.downcast_ref::<T>())
        })
    }

    pub fn get_component_mut<T: Any>(&mut self, entity: &EntityId) -> Option<&mut T> {
        self.map.get_mut(entity).and_then(|components| {
            components
                .get_mut(&TypeId::of::<T>())
                .and_then(|v| v.downcast_mut::<T>())
        })
    }

    pub fn next_id(&mut self) -> EntityId {
        let ret = self.next_id;
        self.next_id += 1;
        ret
    }

    pub fn new_entity(&mut self) -> EntityId {
        let id = self.next_id();
        self.map.insert(id, HashMap::new());
        id
    }

    pub fn delete_entity(&mut self, entity: &EntityId) {
        let _ = self.map.remove(entity);
    }

    pub fn add_component<T: Any>(&mut self, entity: &EntityId, component: T) -> Option<Box<T>> {
        self.map
            .get_mut(entity)
            .and_then(|components| components.insert(TypeId::of::<T>(), Box::new(component)))
            .map(|old_component| old_component.downcast::<T>().expect("Incongruous types"))
    }

    pub fn remove_component<T: Any>(&mut self, entity: &EntityId) -> Option<Box<T>> {
        self.map
            .get_mut(entity)
            .and_then(|components| components.remove(&TypeId::of::<T>()))
            .map(|old_component| old_component.downcast::<T>().expect("Incongruous types"))
    }

    pub fn with(&self, component_ids: &[TypeId]) -> Vec<EntityId> {
        self.map
            .iter()
            .filter_map(|(id, cpts)| {
                if component_ids.iter().all(|cid| cpts.contains_key(&cid)) {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn components<T: Any>(&self) -> impl Iterator<Item = (EntityId, &T)> {
        self.map.iter().filter_map(|(id, cps)| {
            cps.get(&TypeId::of::<T>())
                .and_then(|cpt| cpt.downcast_ref())
                .map(|cpt| (*id, cpt))
        })
    }

    pub fn components_mut<T: Any>(&mut self) -> impl Iterator<Item = (EntityId, &mut T)> {
        self.map.iter_mut().filter_map(|(id, cps)| {
            cps.get_mut(&TypeId::of::<T>())
                .and_then(|cpt| cpt.downcast_mut())
                .map(|cpt| (*id, cpt))
        })
    }
}

struct Renderable {
    id: Id,
    transform: Isometry2<f32>,
    deleted: bool,
}

impl Renderable {
    pub async fn new(renderer: &mut RendererConn<Socket>, shape: ObjectData) -> Self {
        let transform = shape.transform;
        let id = renderer.add_object(shape).await;
        Self {
            id,
            transform,
            deleted: false,
        }
    }

    pub async fn set_transform(
        &mut self,
        renderer: &mut RendererConn<Socket>,
        transform: Isometry2<f32>,
    ) {
        self.transform = transform;
        renderer.set_transform(self.id, transform).await;
    }

    pub fn get_transform(&self) -> Isometry2<f32> {
        self.transform
    }

    pub async fn delete(mut self, renderer: &mut RendererConn<Socket>) {
        renderer.delete_object(self.id).await;
        self.deleted = true;
    }
}

impl Drop for Renderable {
    fn drop(&mut self) {
        if !self.deleted {
            panic!("Must explicitly delete renderable objects");
        }
    }
}

struct Velocity(pub Vector2<f32>);

async fn physics_system(ecs: &mut ECData, renderer: &mut RendererConn<Socket>, bounds: &Box2D) {
    let entities = ecs.with(&[TypeId::of::<Renderable>(), TypeId::of::<Velocity>()]);
    for entity in &entities {
        let velocity = ecs.get_component::<Velocity>(entity).unwrap().0;
        let rdr = ecs.get_component_mut::<Renderable>(entity).unwrap();
        let iso = rdr.get_transform();
        let mut position: Point2<f32> = iso.translation.vector.into();
        position += velocity;
        let position = bounds.wrap(position);
        rdr.set_transform(
            renderer,
            Isometry2::new(position.coords, iso.rotation.angle()),
        )
        .await;
    }
}

// What do you do for a transform change? Send everything every frame?

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

struct Ship;

impl Ship {
    pub async fn new(ecs: &mut ECData, renderer: &mut RendererConn<Socket>) -> EntityId {
        let id = ecs.new_entity();
        let shape = render::ObjectData::new(objects::rocket(), Isometry2::identity());
        ecs.add_component(&id, Renderable::new(renderer, shape).await);
        ecs.add_component(&id, Velocity(Vector2::new(0.0, 0.0)));
        ecs.add_component(&id, Ship);
        id
    }

    pub async fn system(
        ecs: &mut ECData,
        renderer: &mut RendererConn<Socket>,
        fire_engine: bool,
        left_key: bool,
        right_key: bool,
        fire_laser: bool,
    ) {
        let ships = ecs.with(&[
            TypeId::of::<Ship>(),
            TypeId::of::<Renderable>(),
            TypeId::of::<Velocity>(),
        ]);
        for ship in &ships {
            let rdr = ecs.get_component_mut::<Renderable>(ship).unwrap();
            let transform = rdr.get_transform();
            let mut angle = transform.rotation.angle();

            const ROT_RATE: f32 = 0.08;
            if left_key {
                angle += ROT_RATE;
            }
            if right_key {
                angle -= ROT_RATE;
            }

            rdr.set_transform(renderer, Isometry2::new(transform.translation.vector, angle)).await;

            let direction = Vector2::new(angle.cos(), angle.sin());

            const ACCEL: f32 = 0.1;
            let acceleration = if fire_engine {
                direction * ACCEL
            } else {
                Vector2::new(0.0, 0.0)
            };

            let velocity = &mut ecs.get_component_mut::<Velocity>(ship).unwrap().0;
            *velocity += acceleration;
            let velocity = *velocity;

            if fire_laser {
                Bullet::new(
                    ecs,
                    renderer,
                    transform.translation.vector.into(),
                    velocity + (direction * 8.0),
                )
                .await;
            }
        }
    }
}

struct Bullet {
    pub duration: u32,
}

impl Bullet {
    pub async fn new(
        ecs: &mut ECData,
        renderer: &mut RendererConn<Socket>,
        position: Point2<f32>,
        velocity: Vector2<f32>,
    ) -> EntityId {
        let id = ecs.new_entity();
        let shape = render::ObjectData::new(
            objects::bullet(),
            Isometry2::translation(position.x, position.y),
        );
        ecs.add_component(&id, Renderable::new(renderer, shape).await);
        ecs.add_component(&id, Velocity(velocity));
        ecs.add_component(&id, Bullet { duration: 90 });
        id
    }

    pub async fn system(ecs: &mut ECData, renderer: &mut RendererConn<Socket>) {
        let bullets = ecs.with(&[
            TypeId::of::<Bullet>(),
            TypeId::of::<Renderable>(),
        ]);
        for bullet in &bullets {
            let data = ecs.get_component_mut::<Bullet>(bullet).unwrap();
            data.duration -= 1;
            if data.duration <= 0 {
                ecs.remove_component::<Renderable>(bullet)
                    .unwrap()
                    .delete(renderer)
                    .await;
                ecs.delete_entity(bullet);
            }
        }
    }
}

async fn asteroids() {
    let mut ecs = ECData::new();
    let socket = Socket::connect("renderer", 0).unwrap().await.unwrap();
    debug("Connected to renderer");
    let mut renderer = RendererConn::new(socket);

    let screen = Box2D::new(Point2::new(-360.0, -360.0), Point2::new(360.0, 360.0));
    let _screen_rect = renderer
        .add_object(render::ObjectData {
            data: objects::rectangle(&screen, Point3::new(1.0, 1.0, 1.0)),
            transform: Isometry2::identity(),
        })
        .await;

    Ship::new(&mut ecs, &mut renderer).await;
    loop {
        let info = renderer.wait_frame().await;
        Ship::system(
            &mut ecs,
            &mut renderer,
            info.keys.contains(&'W'),
            info.keys.contains(&'A'),
            info.keys.contains(&'D'),
            info.keys.contains(&' '),
        )
        .await;
        Bullet::system(&mut ecs, &mut renderer).await;
        physics_system(&mut ecs, &mut renderer, &screen).await;
    }
}

/*
impl Bullet {
pub async fn new<S: AsyncRead + AsyncWrite + Unpin>(
renderer: &mut RendererConn<S>,
position: Point2<f32>,
velocity: Vector2<f32>,
) -> Self {
let render_id = renderer
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
renderer: &mut RendererConn<S>,
screen: Box2D,
) {
self.position += self.velocity;
self.position = screen.wrap(self.position);
renderer.set_transform(self.render_id, Isometry2::new(self.position.coords, 0.0))
.await;
    self.duration -= 1;
}
}

struct Asteroid {
pub radius: f32,
pub life: u32,
}

impl Asteroid {
pub async fn new<S: AsyncRead + AsyncWrite + Unpin>(
renderer: &mut RendererConn<S>,
position: Point2<f32>,
velocity: Vector2<f32>,
radius: f32,
) -> Self {
let render_id = renderer
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
renderer: &mut RendererConn<S>,
screen: Box2D,
) {
self.position += self.velocity;
self.position = screen.wrap(self.position);
renderer.set_transform(self.render_id, Isometry2::new(self.position.coords, 0.0))
    .await;
}

pub fn collides_with(&self, object: Point2<f32>) -> bool {
    (self.position - object).magnitude() <= self.radius
}
}

    let mut rng = SmallRng::seed_from_u64(3489277);
    let pos = Uniform::new(-300.9, 300.0);
    let size = Uniform::new(10.0, 60.0);
    let vel = Uniform::new(0.1, 1.0);

    for _ in 0..10 {
        asteroids.push(
            Asteroid::new(
                &mut renderer,
                Point2::new(pos.sample(&mut rng), pos.sample(&mut rng)),
                Vector2::new(vel.sample(&mut rng), vel.sample(&mut rng)),
                size.sample(&mut rng),
            )
            .await,
        );
    }
        if info.keys.contains(&' ') {
            bullets.push(
                Bullet::new(
                    &mut renderer,
                    ship.position,
                    ship.velocity + Vector2::new(ship.angle.cos(), ship.angle.sin()) * 8.5,
                )
                .await,
            );
        }
        for bullet in &mut bullets {
            bullet.update(&mut renderer, screen).await;
            if bullet.duration == 0 {
                renderer.delete_object(bullet.render_id).await;
            }
        }
        let mut new_roids = Vec::new();
        for asteroid in &mut asteroids {
            asteroid.update(&mut renderer, screen).await;
            if asteroid.collides_with(ship.position) {
                debug("You died!");
                break 'mainloop;
            }
            for bullet in &mut bullets {
                if asteroid.collides_with(bullet.position) {
                    debug(&format!("HIT {} {}", bullet.position.x, bullet.position.y));
                    bullet.duration = 0;
                    renderer.delete_object(bullet.render_id).await;
                    asteroid.life -= 1;
                }
            }
            if asteroid.life <= 0 {
                renderer.delete_object(asteroid.render_id).await;
                if asteroid.radius > 10.0 {
                    new_roids.push(
                        Asteroid::new(
                            &mut renderer,
                            asteroid.position,
                            asteroid.velocity / 2.0,
                            asteroid.radius / 2.0,
                        )
                        .await,
                    );
                    new_roids.push(
                        Asteroid::new(
                            &mut renderer,
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
*/
