use game_kernel::manager::Manager;
use game_kernel::native_module::NativeModule;
//use game_kernel::wasm_module::WasmModule;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Loading instances...");
    let mut manager = Manager::new();

    manager.add_module(
        "plugin_a",
        Box::new(NativeModule::from_path(
            "../plugin_a/target/x86_64-unknown-linux-gnu/debug/libplugin_a.so",
        )?),
    );

    manager.add_module(
        "plugin_b",
        Box::new(NativeModule::from_path(
            "../plugin_b/target/x86_64-unknown-linux-gnu/debug/libplugin_b.so",
        )?),
    );

    println!("Running:");
    for _ in 0..400000 {
        manager.run();
    }

    Ok(())
}
