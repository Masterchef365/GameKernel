#[cfg(feature = "host")]
mod host;

#[cfg(feature = "host")]
pub use host::*;

use futures::{AsyncRead, AsyncWrite, SinkExt, StreamExt};
use nalgebra::{Point3, Translation3};
use serde::{Deserialize, Serialize};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tokio_util::compat::Compat;
use tokio_util::compat::FuturesAsyncReadCompatExt;

pub type Id = u64;

#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectData {
    pub data: Box<[(Point3<f32>, Point3<f32>, Point3<f32>)]>,
    pub transform: Translation3<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    CreateObject(ObjectData),
    SetObjectTranslation(Id, Translation3<f32>),
    DeleteObject(Id),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    ObjectCreated(Id),
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
            .send(
                bincode::serialize(&Request::CreateObject(object))
                    .unwrap()
                    .into(),
            )
            .await
            .unwrap();
        let Response::ObjectCreated(id) =
            bincode::deserialize(&self.socket.next().await.unwrap().unwrap()).unwrap();
        id
    }

    pub async fn set_transform(&mut self, id: Id, transform: Translation3<f32>) {
        self.socket
            .send(
                bincode::serialize(&Request::SetObjectTranslation(id, transform))
                    .unwrap()
                    .into(),
            )
            .await
            .unwrap();
    }
}
