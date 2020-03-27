use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use wasmer_runtime::{error, func, imports, instantiate, Array, Func, WasmPtr, Ctx};

const WASM_PATH: &str = "../plugin/target/wasm32-unknown-unknown/release/plugin.wasm";
fn main() -> Result<(), Box<dyn Error>> {
    println!("Loading instance...");
    let mut wasm = Vec::new();

    File::open(WASM_PATH)?.read_to_end(&mut wasm)?;

    let import_object = imports! {
        "env" => {
            "write" => func!(|ctx: &mut Ctx, fd: u32, buf: WasmPtr<u8, Array>, len: u32| {
                let buf = buf.deref(ctx.memory(0), 0, len).unwrap();
                for elem in buf {
                    print!("{}", elem.get() as char);
                }
                println!();
                len as i64
            }),
        },
    };

    let mut instance = instantiate(&wasm, &import_object)?;

    println!("Executing...");
    let main_func: Func = instance.func("main")?;

    main_func.call()?;

    Ok(())
}
