use crate::*;
use futures::lock::Mutex;
use futures::{AsyncRead, AsyncWrite, SinkExt, StreamExt};
use kiss3d::event::{Action, Key};
use kiss3d::window::Window;
use std::collections::HashMap;
use std::sync::Arc;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tokio_util::compat::FuturesAsyncReadCompatExt;

pub struct Renderer {
    next_id: Id,
    objects: HashMap<Id, ObjectData>,
    waiting_for_frame: Vec<futures::channel::oneshot::Sender<FrameInfo>>,
}

impl Renderer {
    pub fn new(window_name: String) -> Arc<Mutex<Self>> {
        let instance = Arc::new(Mutex::new(Self {
            next_id: 0,
            objects: HashMap::new(),
            waiting_for_frame: Vec::new(),
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
                Request::WaitFrame => {
                    // Set a wakeup routine on the renderer
                    let (tx, rx) = futures::channel::oneshot::channel();
                    share.waiting_for_frame.push(tx);

                    // Release the lock so the renderer can actually render the frame
                    drop(share);

                    // Wake back up when there's data and pass it back to the client
                    framed
                        .send(bincode::serialize(&rx.await.unwrap()).unwrap().into())
                        .await
                        .unwrap();
                }
                Request::DeleteObject(id) => {
                    share.objects.remove(&id);
                }
                Request::CreateObject(object) => {
                    let id = share.next_id();
                    share.objects.insert(id, object);
                    drop(share); // Release the lock early
                    framed
                        .send(bincode::serialize(&id).unwrap().into())
                        .await
                        .unwrap();
                }
                Request::SetObjectTranslation(id, transform) => {
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
            let mut share = loop {
                if let Some(lock) = share.try_lock() {
                    break lock;
                }
                std::thread::yield_now();
            };

            for object in share.objects.values() {
                for (a, b, color) in object.data.iter() {
                    let a = object.transform.transform_point(a);
                    let b = object.transform.transform_point(b);
                    window.draw_line(&a, &b, &color);
                }
            }

            // Wake up registered events
            for waiter in share.waiting_for_frame.drain(..) {
                let mut keys = Vec::new();
                for (key, response) in
                    [(Key::W, 'W'), (Key::A, 'A'), (Key::S, 'S'), (Key::D, 'D')].iter()
                {
                    match window.get_key(*key) {
                        Action::Press => keys.push(*response),
                        _ => (),
                    }
                }
                waiter.send(FrameInfo { keys }).unwrap();
            }
        }
    }
}
