mod debug;
mod maybe;
mod reactor;
mod socket_types;
mod task_pool;
pub use debug::debug;
#[cfg(target_arch = "wasm32")]
pub use socket::{Socket, SocketListener};
pub use task_pool::{spawn, yield_now};

pub use futures::io::{AsyncReadExt, AsyncWriteExt};
pub use futures::stream::StreamExt;

#[cfg(target_arch = "wasm32")]
pub mod wasm_socket;
#[cfg(target_arch = "wasm32")]
pub use wasm_socket as socket;

//#[cfg(not(target_arch = "wasm32"))]
//pub mod native_socket;
//#[cfg(not(target_arch = "wasm32"))]
//pub use native_socket as socket;
