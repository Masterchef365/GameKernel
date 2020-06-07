#![allow(unused_imports)]
mod wasm_module;
use anyhow::{format_err, Result};
use futures::executor::ThreadPool;
use futures::task::SpawnExt;
use futures::{SinkExt, StreamExt};
use host::matchmaker::{self, MatchMakerConnection};
use std::error::Error;
use std::fs::{create_dir, read_dir};
use std::path::Path;
use wasm_module::WasmModule;

fn load_mods(
    folder: impl AsRef<Path>,
    executor: &impl SpawnExt,
    matchmaker: MatchMakerConnection,
) -> Result<()> {
    let mods_folder = read_dir(&folder);
    if let Err(e) = &mods_folder {
        if e.kind() == std::io::ErrorKind::NotFound {
            eprintln!("Mods folder not found; created.");
            create_dir(folder)?;
            return Ok(());
        }
    }

    for file in mods_folder? {
        let file = file?;

        let ftype = file.file_type()?;
        let path = if ftype.is_file() {
            file.path()
        } else if ftype.is_symlink() {
            file.path().read_link()?
        } else {
            continue;
        };

        let id = path.file_stem().unwrap().to_str().unwrap().into();
        executor.spawn(WasmModule::from_path(path)?.task(id, matchmaker.clone()))?;
    }

    Ok(())
}

fn main() -> Result<()> {
    // Set up the thread pool and essential tasks
    let spawner = ThreadPool::new()?;
    let (mm, tx) = matchmaker::MatchMaker::new();
    spawner.spawn(mm.task())?;

    // Load user-written mods
    println!("Loading mods...");
    load_mods("../mods", &spawner, tx.clone())?;

    // Spawn native-code tasks
    spawner.spawn(vg_server(tx.clone(), spawner.clone()))?;

    // Let the executor take over from here
    Ok(std::thread::park())
}

async fn vg_server(mut mm: matchmaker::MatchMakerConnection, spawner: ThreadPool) {
    use tokio_util::codec::{Framed, LengthDelimitedCodec};
    use tokio_util::compat::FuturesAsyncReadCompatExt;
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
