mod socket;
pub use socket::Socket;
mod task_pool;
pub use task_pool::spawn;
mod reactor;

type Handle = u32;
