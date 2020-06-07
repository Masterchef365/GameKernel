#[cfg(feature = "host")]
mod host;

#[cfg(feature = "host")]
pub use host::*;

use futures::{AsyncRead, AsyncWrite, SinkExt, StreamExt};
pub use nalgebra::{Isometry2, Point2, Point3, Vector2, Vector3, Rotation2};
use serde::{Deserialize, Serialize};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tokio_util::compat::Compat;
use tokio_util::compat::FuturesAsyncReadCompatExt;

pub type Line = (Point2<f32>, Point2<f32>, Point3<f32>);

#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectData {
    pub data: Box<[Line]>,
    pub transform: Isometry2<f32>,
}

pub type Id = u64;

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    CreateObject(ObjectData),
    SetObjectTranslation(Id, Isometry2<f32>),
    DeleteObject(Id),
    WaitFrame,
}

pub struct RendererConn<S> {
    socket: Framed<Compat<S>, LengthDelimitedCodec>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FrameInfo {
    pub keys: Vec<char>,
}

impl ObjectData {
    pub fn new(data: Box<[Line]>, transform: Isometry2<f32>) -> Self {
        Self {
            data, transform
        }
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> RendererConn<S> {
    pub fn new(socket: S) -> Self {
        Self {
            socket: Framed::new(socket.compat(), LengthDelimitedCodec::new()),
        }
    }

    // TODO: Make this fallible
    pub async fn add_object(&mut self, object: ObjectData) -> Id {
        self.socket
            .send(
                bincode::serialize(&Request::CreateObject(object))
                    .unwrap()
                    .into(),
            )
            .await
            .unwrap();
        bincode::deserialize(&self.socket.next().await.unwrap().unwrap()).unwrap()
    }

    // TODO: Make this fallible
    pub async fn delete_object(&mut self, id: Id) {
        self.socket
            .send(
                bincode::serialize(&Request::DeleteObject(id))
                    .unwrap()
                    .into(),
            )
            .await
            .unwrap();
    }

    pub async fn set_transform(&mut self, id: Id, transform: Isometry2<f32>) {
        self.socket
            .send(
                bincode::serialize(&Request::SetObjectTranslation(id, transform))
                    .unwrap()
                    .into(),
            )
            .await
            .unwrap();
    }

    pub async fn wait_frame(&mut self) -> FrameInfo {
        self.socket
            .send(bincode::serialize(&Request::WaitFrame).unwrap().into())
            .await
            .unwrap();
        let msg = self.socket.next().await.unwrap().unwrap();
        bincode::deserialize(&msg).unwrap()
    }
}
