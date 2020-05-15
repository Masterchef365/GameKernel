mod wasm_module;
use anyhow::Result;
use futures::executor::ThreadPool;
use futures::task::SpawnExt;
use futures::{SinkExt, StreamExt};
use game_kernel::matchmaker;
use wasm_module::WasmModule;

use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tokio_util::compat::FuturesAsyncReadCompatExt;

use std::sync::Arc;

fn main() -> Result<()> {
    let mut spawner = ThreadPool::new()?;

    let (mm, tx) = matchmaker::MatchMaker::new();
    spawner.spawn(mm.task())?;

    //let plugin_a = WasmModule::from_path("/home/duncan/Projects/game_kernel/plugin_a/target/wasm32-unknown-unknown/release/plugin_a.wasm")?;
    //spawner.spawn(plugin_a.task("plugin_a".into(), tx.clone()))?;

    let plugin_b = WasmModule::from_path("/home/duncan/Projects/game_kernel/plugin_b/target/wasm32-unknown-unknown/release/plugin_b.wasm")?;
    spawner.spawn(plugin_b.task("plugin_b".into(), tx.clone()))?;

    //for i in 0..5 {
        spawner.spawn(test_task(tx.clone(), spawner.clone()))?;
    //}

    Ok(std::thread::park())
}

async fn test_task(mut mm: matchmaker::MatchMakerConnection, spawner: ThreadPool) {
    println!("Test task started");
    let mut conn = matchmaker::create_listener("plugin_a", 5062, &mut mm).await.unwrap();
    while let Some(socket) = conn.next().await {
        println!("Got new connection");
        spawner.spawn(async move {
            let mut framed = Framed::new(socket.compat(), LengthDelimitedCodec::new());
            println!("Server handling new connection");
            loop {
                let bytes = framed.next().await.unwrap().unwrap();
                println!("{}:", &String::from_utf8(bytes.to_vec()).unwrap());
                framed.send("Message from server".into()).await.unwrap();
            }
        }).unwrap();
    }
}


/*
   async fn test_task(mut mm: matchmaker::MMSender, name: String) {
   let mut conn = matchmaker::connect("plugin_a", 5062, &mut mm)
   .await
   .expect("No option")
   .expect("No socket");
   use tokio_util::codec::{Framed, LengthDelimitedCodec};
   use tokio_util::compat::FuturesAsyncReadCompatExt;
   let mut socket = Framed::new(conn.compat(), LengthDelimitedCodec::new());
   let mut n = 0u32;
   loop {
   socket
   .send(format!("{}: {}", name, n).into())
   .await
   .unwrap();
   println!("{}: {:?}", name, socket.next().await.unwrap().unwrap());
   n += 1;
   }
   }
   */
