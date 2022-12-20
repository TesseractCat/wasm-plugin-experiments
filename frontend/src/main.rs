use std::error::Error;
use wasmtime::*;
use serde::{Serialize, Deserialize};
use rmp_serde::{to_vec_named, from_read};

fn rpc<S: AsContextMut, T: Serialize, R: for<'a> Deserialize<'a>>(instance: &Instance, mut store: S, func: &str, args: T)
                                      -> Result<R, Box<dyn Error>> {
    // Serialize arguments
    let buffer = to_vec_named(&args)?;
    //println!("Serialized buffer: {:?}", buffer);

    // Allocate memory for arguments
    let alloc = instance.get_typed_func::<i32, i32, &mut S>(&mut store, "alloc")?;
    let offset = alloc.call(&mut store, buffer.len() as i32)?;
    //println!("Allocated at: {:?}", offset);

    // Get memory and write msgpack arguments
    let memory = instance.get_memory(&mut store, "memory").ok_or::<Box<dyn Error>>("Failed to get memory".into())?;
    memory.write(&mut store, offset as usize, &buffer)?;

    // Call function
    let f = instance.get_typed_func::<(i32, i32), u32, &mut S>(&mut store, &(String::from("client_") + func))?;
    let result = f.call(&mut store, (offset, buffer.len() as i32))? as usize;
    if result == 0 {
        return Err("Failed to call RPC".into());
    }

    // Get return slice
    let len = u32::from_le_bytes(memory.data(&mut store)[result..(result + 4)].try_into()?) as usize;
    let data = &memory.data(&mut store)[(result+4)..(result+4+len)];
    //println!("Returned at [{:?}] with length [{:?}]: {:?}", result, len, data);

    // Deserialize results
    let deserialized = from_read(data)?;
    Ok(deserialized)
}
fn rpc_wrap<P: for<'a> Deserialize<'a>, T: Serialize>(
    func: fn(P) -> T) -> impl Fn(Caller<'_, ()>, i32, i32) -> i32
{
    move |mut caller: Caller<'_, ()>, ptr: i32, len: i32| -> i32 {
        let ptr = ptr as usize;
        let len = len as usize;
        let mut memory = caller.get_export("memory").unwrap().into_memory().unwrap();
        let mut store = caller.as_context_mut();

        // Deserialize parameters from linear memory
        let data = &memory.data(&mut store)[ptr..(ptr + len)];
        let deserialized: P = from_read(data).unwrap();

        // Call function and serialize output
        let result = func(deserialized);
        let serialized = rmp_serde::to_vec_named(&result);

        0
    }
}

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
    let module = Module::from_file(&engine,
                                   "../apps/addition/target/wasm32-unknown-unknown/debug/addition.wasm")?;
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker.func_wrap("env", "host_print", rpc_wrap(wasm_print))?;
    let instance = linker.instantiate(&mut store, &module)?;

    let bob = Person { name: "Bob".to_owned(), age: 12, cool: true };
    println!("Result: {:?}",
             rpc::<_, _, bool>(&instance, &mut store, "extract", (bob,))
    );
    rpc::<_, _, ()>(&instance, &mut store, "echo", "Cool dog wearing cool hat")?;

    Ok(())
}
