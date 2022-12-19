use std::error::Error;
use wasmtime::*;
use serde::{Serialize, Deserialize};
use rmp_serde::{to_vec, from_read};

fn rpc<S: AsContextMut, T: Serialize, R: for<'a> Deserialize<'a>>(instance: &Instance, mut store: S, func: &str, args: T)
                                      -> Result<R, Box<dyn Error>> {
    // Serialize arguments
    let buffer = to_vec(&args)?;

    // Allocate memory for arguments
    let alloc = instance.get_typed_func::<i32, i32, &mut S>(&mut store, "alloc")?;
    let offset = alloc.call(&mut store, buffer.len() as i32)?;
    //println!("Allocated at: {:?}", offset);

    // Get memory and write msgpack arguments
    let memory = instance.get_memory(&mut store, "memory").ok_or::<Box<dyn Error>>("Failed to get memory".into())?;
    memory.write(&mut store, offset as usize, &buffer)?;

    // Call function
    let f = instance.get_typed_func::<(i32, i32), u32, &mut S>(&mut store, func)?;
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

fn main() -> Result<(), Box<dyn Error>> {
    let engine = Engine::default();
    let module = Module::from_file(&engine,
                                   "plugins/addition/target/wasm32-unknown-unknown/debug/addition.wasm")?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    println!("Result: {:?}",
             rpc::<_, _, ()>(&instance, &mut store, "duck", (1, 2))
    );

    Ok(())
}
