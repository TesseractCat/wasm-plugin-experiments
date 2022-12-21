#![feature(fn_traits, unboxed_closures, tuple_trait)]

mod rpc;
use rpc::*;

use std::error::Error;
use wasmtime::*;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Person {
    name: String,
    age: u32,
    cool: bool
}

fn wasm_print(text: String) {
    println!("{}", text);
}

fn main() -> Result<(), Box<dyn Error>> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, AppWasmData { alloc: None });
    let mut linker = Linker::<AppWasmData>::new(&engine);
    linker.func_wrap("env", "print", rpc_wrap(wasm_print))?;

    let module = Module::from_file(
        &engine, "../apps/addition/target/wasm32-unknown-unknown/debug/addition.wasm"
    )?;
    let instance = linker.instantiate(&mut store, &module)?;
    store.data_mut().alloc = Some(instance.get_typed_func::<i32, i32, _>(&mut store, "alloc")?);

    let bob = Person { name: "Bob".to_owned(), age: 12, cool: true };
    println!("Result: {:?}",
             instance.rpc::<_, String>(&mut store, "extract", (bob,))
    );
    instance.rpc::<_, ()>(&mut store, "echo", ("Cool dog wearing cool hat",))?;

    Ok(())
}
