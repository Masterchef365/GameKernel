use host::manager::Manager;
use std::error::Error;
//use wasm_module::WasmModule;
use host::native_module::NativeModule;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Loading instances...");
    let mut manager = Manager::new();
    let mut native_module = NativeModule::new();
    native_module.spawn(async {
        println!("Nice work bucko!");
    });
    manager.add_module("native", Box::new(native_module));

    /*
    manager.add_module(
        "plugin_a",
        Box::new(WasmModule::from_path(
            "../plugin_a/target/wasm32-unknown-unknown/release/plugin_a.wasm",
        )?),
    );
    manager.add_module(
        "plugin_b",
        Box::new(WasmModule::from_path(
            "../plugin_b/target/wasm32-unknown-unknown/release/plugin_b.wasm",
        )?),
    );
    */

    println!("Running:");
    for _ in 0..400000 {
        manager.run();
    }

    Ok(())
}
