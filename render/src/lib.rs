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

pub type ObjectData;

pub type Id = u64;

pub enum Request {
    CreateObject,
    SetObjectData(Id, ObjectData),
    DeleteObject(Id),
}

pub enum Response {
    ObjectCreated(Id),
}

pub struct Renderer {
    window: Window,
    next_id: Id,
    objects: HashMap<Id, ObjectData>,
}

impl Renderer {
    pub fn new(window_name: &str) -> Self {
        Self {
            next_id: 0,
            objects: Default::default(),
        }
    }

    pub fn loop()

    pub fn spawn() {
        let renderer = Renderer::new()
    }
}
