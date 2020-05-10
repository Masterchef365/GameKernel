mod wasm_module;
use wasm_module::WasmModule;
use anyhow::Result;
//use futures::executor::ThreadPool;
use futures::task::SpawnExt;
use futures::executor::LocalPool;
use game_kernel::matchmaker::MatchMaker;
fn main() -> Result<()> {
    let mut pool = LocalPool::new();
    let spawner = pool.spawner();

    let (mm, tx) = MatchMaker::new();
    spawner.spawn(mm.task())?;

    let plugin_a = WasmModule::from_path("/home/duncan/Projects/game_kernel/plugin_a/target/wasm32-unknown-unknown/release/plugin_a.wasm")?;
    spawner.spawn(plugin_a.task("plugin_a".into(), tx.clone()))?;

    let plugin_b = WasmModule::from_path("/home/duncan/Projects/game_kernel/plugin_b/target/wasm32-unknown-unknown/release/plugin_b.wasm")?;
    spawner.spawn(plugin_b.task("plugin_b".into(), tx.clone()))?;
    pool.run();
    Ok(std::thread::park())
}

/*
#[macro_use]
extern crate rental;
use game_kernel::executor::{Executor, Module};
use game_kernel::matchmaker::MatchMaker;
use native_module::NativeModule;
use std::error::Error;
use std::fs::{create_dir, read_dir};
use std::io::ErrorKind;
use std::path::Path;
use std::sync::mpsc::channel;
use wasm_module::WasmModule;
mod wasm_module;
mod native_module;
fn load_by_name(path: impl AsRef<Path>) -> Result<Box<dyn Module>, Box<dyn Error>> {
    let path = path.as_ref();
    match path.extension().unwrap().to_str().unwrap() {
        "wasm" => Ok(Box::new(WasmModule::from_path(path)?)),
        "so" => Ok(Box::new(NativeModule::from_path(path)?)),
        ext => Err(format!("Unrecognized plugin extension '{:?}'", ext).into()),
    }
}

fn path_to_name(path: impl AsRef<Path>) -> String {
    path.as_ref().file_stem().unwrap().to_str().unwrap().into()
}

const MODS_PATH: &str = "mods";
fn main() -> Result<(), Box<dyn Error>> {
    let mods_folder = read_dir(MODS_PATH);
    if let Err(e) = &mods_folder {
        if e.kind() == ErrorKind::NotFound {
            eprintln!("Mods folder not found; created.");
            create_dir(MODS_PATH)?;
            return Ok(());
        }
    }

    println!("Initializing manager...");
    let (match_tx, match_rx) = channel();
    let mut executor = Executor::new(match_tx);
    let mut matchmaker = MatchMaker::new(match_rx, executor.sender());

    println!("Loading mods...");
    for file in mods_folder? {
        let file = file?;
        let ftype = file.file_type()?;
        if ftype.is_file() {
            executor.add_module(path_to_name(file.path()), load_by_name(file.path())?);
        }
        if ftype.is_symlink() {
            let path = file.path().read_link()?;
            executor.add_module(path_to_name(path.clone()), load_by_name(path)?);
        }
    }

    println!("Running:");
    loop {
        matchmaker.run();
        executor.run();
        std::thread::yield_now();
    }
}
*/
