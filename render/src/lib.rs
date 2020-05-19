use futures::lock::Mutex;
use futures::{AsyncRead, AsyncWrite, SinkExt, StreamExt};
use kiss3d::window::Window;
use nalgebra::{Point3, Transform3};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tokio_util::compat::Compat;
use tokio_util::compat::FuturesAsyncReadCompatExt;

#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectData {
    pub data: Box<[(Point3<f32>, Point3<f32>, Point3<f32>)]>,
    pub transform: Transform3<f32>,
}

pub type Id = u64;

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    CreateObject(ObjectData),
    SetObjectTransform(Id, Transform3<f32>),
    DeleteObject(Id),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    ObjectCreated(Id),
}

pub struct Renderer {
    next_id: Id,
    objects: HashMap<Id, ObjectData>,
}

impl Renderer {
    pub fn new(window_name: String) -> Arc<Mutex<Self>> {
        let instance = Arc::new(Mutex::new(Self {
            next_id: 0,
            objects: HashMap::new(),
        }));
        let ret = instance.clone();
        std::thread::spawn(move || Self::render_loop(instance, window_name));
        ret
    }

    pub fn next_id(&mut self) -> Id {
        let ret = self.next_id;
        self.next_id += 1;
        ret
    }

    /// It is advisable to put this in its own task
    pub async fn handle_client(
        share: Arc<Mutex<Self>>,
        socket: impl AsyncRead + AsyncWrite + Unpin,
    ) {
        let mut framed = Framed::new(socket.compat(), LengthDelimitedCodec::new());
        while let Some(Ok(msg)) = framed.next().await {
            let request: Request = bincode::deserialize(&msg).unwrap();
            let mut share = share.lock().await;
            match request {
                Request::DeleteObject(id) => {
                    share.objects.remove(&id);
                }
                Request::CreateObject(object) => {
                    let id = share.next_id();
                    share.objects.insert(id, object);
                    drop(share); // Release the lock early
                    framed
                        .send(
                            bincode::serialize(&Response::ObjectCreated(id))
                                .unwrap()
                                .into(),
                        )
                        .await
                        .unwrap();
                }
                Request::SetObjectTransform(id, transform) => {
                    if let Some(object) = share.objects.get_mut(&id) {
                        object.transform = transform;
                    }
                }
            }
        }
    }

    pub fn render_loop(share: Arc<Mutex<Self>>, window_name: String) {
        let mut window = Window::new(&window_name);
        while window.render() {
            for object in share.try_lock().unwrap().objects.values() {
                for (a, b, color) in object.data.iter() {
                    let a = object.transform.transform_point(a);
                    let b = object.transform.transform_point(b);
                    window.draw_line(&a, &b, &color);
                }
            }
        }
    }
}

pub struct RendererConnection<S> {
    socket: Framed<Compat<S>, LengthDelimitedCodec>,
}

impl<S: AsyncRead + AsyncWrite + Unpin> RendererConnection<S> {
    pub fn new(socket: S) -> Self {
        Self {
            socket: Framed::new(socket.compat(), LengthDelimitedCodec::new()),
        }
    }

    pub async fn add_object(&mut self, object: ObjectData) -> Id {
        self.socket
            .send(bincode::serialize(&Request::CreateObject(object)).unwrap().into())
            .await
            .unwrap();
        let Response::ObjectCreated(id) =
            bincode::deserialize(&self.socket.next().await.unwrap().unwrap()).unwrap();
        id
    }

    pub async fn set_transform(&mut self, id: Id, transform: Transform3<f32>) {
        self.socket
            .send(
                bincode::serialize(&Request::SetObjectTransform(id, transform))
                    .unwrap()
                    .into(),
            )
            .await
            .unwrap();
    }
}
