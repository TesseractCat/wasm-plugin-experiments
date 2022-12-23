#![feature(fn_traits, unboxed_closures, tuple_trait)]

mod rpc;
use rpc::*;

use std::error::Error;
use wasmtime::*;
use serde::{Serialize, Deserialize};
use rmpv::Value;

fn wasm_print(caller: &mut Caller<'_, AppData>, text: String) {
    println!("{}", text);
}
fn wasm_set_output(caller: &mut Caller<'_, AppData>,
                   output: String, data: Value) {
    println!("[{:?}] = {}", output, data);

    *caller.data_mut().outputs
        .entry(output).or_insert(Value::Nil) = data;
}

fn main() -> Result<(), Box<dyn Error>> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, Default::default());
    let mut linker = Linker::<AppData>::new(&engine);
    linker.func_wrap("env", "print", rpc_wrap(wasm_print))?;
    linker.func_wrap("env", "set_output", rpc_wrap(wasm_set_output))?;

    let module = Module::from_file(
        &engine, "../apps/addition/target/wasm32-unknown-unknown/debug/addition.wasm"
    )?;
    let instance = linker.instantiate(&mut store, &module)?;
    store.data_mut().alloc = Some(instance.get_typed_func::<i32, i32, _>(&mut store, "alloc")?);

    instance.rpc::<_, ()>(&mut store, "update", ())?;

    Ok(())
}
