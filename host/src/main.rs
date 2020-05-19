mod wasm_module;
use anyhow::Result;
use futures::executor::ThreadPool;
use futures::task::SpawnExt;
use futures::{SinkExt, StreamExt};
use game_kernel::matchmaker;
use wasm_module::WasmModule;

use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tokio_util::compat::FuturesAsyncReadCompatExt;

fn main() -> Result<()> {
    let spawner = ThreadPool::new()?;

    let (mm, tx) = matchmaker::MatchMaker::new();
    spawner.spawn(mm.task())?;

    /*
    let plugin_a = WasmModule::from_path("../target/wasm32-unknown-unknown/release/plugin_a.wasm")?;
    spawner.spawn(plugin_a.task("plugin_a".into(), tx.clone()))?;
    */

    let plugin_b = WasmModule::from_path("../target/wasm32-unknown-unknown/release/plugin_b.wasm")?;
    spawner.spawn(plugin_b.task("plugin_b".into(), tx.clone()))?;

    spawner.spawn(vg_server(tx.clone(), spawner.clone()))?;
    /*
    for _ in 0..1000 {
    spawner.spawn(test_client(tx.clone()))?;
    }
    */

    Ok(std::thread::park())
}

async fn vg_server(mut mm: matchmaker::MatchMakerConnection, spawner: ThreadPool) {
    let renderer = render::Renderer::new("Game Kernel Vector Graphics".into());
    let mut conn = matchmaker::create_listener("renderer", 0, &mut mm)
        .await
        .unwrap();
    while let Some(socket) = conn.next().await {
        let renderer = renderer.clone();
        spawner
            .spawn(render::Renderer::handle_client(renderer.clone(), socket))
            .unwrap();
    }
}

async fn test_client(mut mm: matchmaker::MatchMakerConnection) {
    let conn = matchmaker::connect("renderer", 0, &mut mm)
        .await
        .expect("No option")
        .expect("No socket");
    let mut conn = render::RendererConnection::new(conn);
    let id = conn
        .add_object(render::ObjectData {
            data: Box::new([(
                render::Point3::origin(),
                render::Point3::new(1.0, 1.0, 1.0),
                render::Point3::new(1.0, 0.5, 1.0),
            )]),
            transform: render::Translation3::identity(),
        })
        .await;
    let mut i: f32 = 0.0;
    loop {
        conn.set_transform(id, render::Translation3::new(i.cos(), 0.0, 0.0))
            .await;
        i += 0.00001;
    }
}
