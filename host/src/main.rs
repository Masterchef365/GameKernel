mod manager;
mod module;
mod socket;
use manager::Manager;
use module::Module;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Loading instances...");
    let mut manager = Manager::new();
    manager.add_module(
        "plugin_a",
        Module::from_path("../plugin_a/target/wasm32-unknown-unknown/release/plugin_a.wasm")?,
    );
    manager.add_module(
        "plugin_b",
        Module::from_path("../plugin_b/target/wasm32-unknown-unknown/release/plugin_b.wasm")?,
    );

    println!("Running:");
    for _ in 0..400000 {
        manager.run();
    }

    Ok(())
}
