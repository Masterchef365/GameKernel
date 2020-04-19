#[macro_use]
extern crate rental;
mod manager;
mod module;
mod native_module;
mod wasm_module;
use manager::Manager;
use native_module::NativeModule;
use std::error::Error;
use std::fs::{create_dir, read_dir, read_link};
use std::io::ErrorKind;
use std::path::Path;
use wasm_module::WasmModule;

fn load_by_name(path: impl AsRef<Path>, manager: &mut Manager) -> Result<(), Box<dyn Error>> {
    let path = path.as_ref();
    let file_name = path.file_stem().unwrap().to_str().unwrap();
    println!("Loading {}", file_name);
    match path.extension().unwrap().to_str().unwrap() {
        "wasm" => Ok(manager.add_module(file_name, Box::new(WasmModule::from_path(path)?))),
        "so" => Ok(manager.add_module(file_name, Box::new(NativeModule::from_path(path)?))),
        ext => Err(format!("Unrecognized plugin extension '{:?}'", ext).into()),
    }
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
    let mut manager = Manager::new();

    println!("Loading mods...");
    for file in mods_folder? {
        let file = file?;
        let ftype = file.file_type()?;
        if ftype.is_file() {
            load_by_name(file.path(), &mut manager)?;
        }
        if ftype.is_symlink() {
            load_by_name(read_link(file.path())?, &mut manager)?;
        }
    }

    println!("Running:");
    for _ in 0..400 {
        manager.run();
    }

    Ok(())
}
