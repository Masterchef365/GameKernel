mod wasm_module;
use anyhow::Result;
use wasm_module::WasmModule;
use futures::executor::LocalPool;
use futures::task::SpawnExt;
use game_kernel::matchmaker;
use futures::{StreamExt, SinkExt};
fn main() -> Result<()> {
    let mut pool = LocalPool::new();
    let spawner = pool.spawner();

    let (mm, tx) = matchmaker::MatchMaker::new();
    spawner.spawn(mm.task())?;

    let plugin_a = WasmModule::from_path("/home/duncan/Projects/game_kernel/plugin_a/target/wasm32-unknown-unknown/release/plugin_a.wasm")?;
    spawner.spawn(plugin_a.task("plugin_a".into(), tx.clone()))?;

    //let plugin_b = WasmModule::from_path("/home/duncan/Projects/game_kernel/plugin_b/target/wasm32-unknown-unknown/release/plugin_b.wasm")?;
    //spawner.spawn(plugin_b.task("plugin_b".into(), tx.clone()))?;
    spawner.spawn(task_1(tx.clone()))?;
    pool.run();
    Ok(std::thread::park())
}

async fn task_1(mut mm: matchmaker::MMSender) {
    let mut conn = matchmaker::connect("plugin_a", 5062, &mut mm).await.unwrap().unwrap();
    use tokio_util::codec::{Framed, LengthDelimitedCodec};
    use tokio_util::compat::FuturesAsyncReadCompatExt;
    let mut socket = Framed::new(conn.compat(), LengthDelimitedCodec::new());
    let mut n = 0u32;
    loop {
        println!("SEND {}", n);
        socket.send(format!("n: {}", n).into()).await.unwrap();
        println!("WAITING");
        println!("{:?}", socket.next().await.unwrap().unwrap());
        n += 1;
    }
}
