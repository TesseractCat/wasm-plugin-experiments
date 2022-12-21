use std::error::Error;
use std::marker::Tuple;
use std::fmt;
use wasmtime::*;
use serde::{Serialize, Deserialize};
use rmp_serde::{to_vec_named, from_read};

#[derive(Copy, Clone, Debug)]
pub enum RpcError {
    ConflictingReturnType,
    ConflictingArgumentType,
    Allocation,
    Panicked,
    Failed,
}
impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}
impl Error for RpcError {}

pub struct AppWasmData {
    pub alloc: Option<TypedFunc<i32, i32>>,
}

pub trait InstanceRpc {
    fn rpc<T, R>(
        &self, store: &mut Store<AppWasmData>, func: &str, args: T
    ) -> Result<R, RpcError>
    where T: Serialize,
          R: for<'a> Deserialize<'a>;
}
impl InstanceRpc for Instance {
    fn rpc<T, R>(
        &self, mut store: &mut Store<AppWasmData>, func: &str, args: T
    ) -> Result<R, RpcError>
    where T: Serialize,
          R: for<'a> Deserialize<'a>
    {
        // Serialize arguments
        let buffer = to_vec_named(&args).map_err(|_| RpcError::Failed)?;
        // Allocate memory for arguments
        let alloc = store.data().alloc.unwrap();
        let offset = alloc.call(&mut store, buffer.len() as i32).map_err(|_| RpcError::Allocation)?;
        // Get memory and write msgpack arguments
        let memory = self.get_memory(&mut store, "memory").ok_or::<Box<dyn Error>>("Failed to get memory".into()).map_err(|_| RpcError::Failed)?;
        memory.write(&mut store, offset as usize, &buffer).map_err(|_| RpcError::Failed)?;
        // Call function
        let f = self.get_typed_func::<(i32, i32), u32, _>(&mut store, &(String::from("client_") + func)).map_err(|_| RpcError::Failed)?;
        let result = f.call(&mut store, (offset, buffer.len() as i32)).map_err(|_| RpcError::Panicked)? as usize;
        if result == 0 {
            return Err(RpcError::ConflictingArgumentType);
        }
        // Get return slice
        let mut buffer = [0u8; 4];
        memory.read(&mut store, result, &mut buffer).expect("Failed to read data");
        let len = u32::from_le_bytes(buffer) as usize;
        let mut data = vec![0; len];
        memory.read(&mut store, result + 4, &mut data).expect("Failed to read data");
        // Deserialize results
        let deserialized = from_read(data.as_slice()).map_err(|_| RpcError::ConflictingReturnType)?;
        Ok(deserialized)
    }
}

pub fn rpc_wrap<P, T>(
    func: impl Fn<P, Output = T>
) -> impl Fn(Caller<'_, AppWasmData>, i32, i32) -> i32
where P: for<'a> Deserialize<'a> + Tuple,
      T: Serialize
{
    move |mut caller: Caller<'_, AppWasmData>, ptr: i32, len: i32| -> i32 {
        let ptr = ptr as usize;
        let len = len as usize;

        let memory = caller.get_export("memory")
            .expect("Failed to get memory export")
            .into_memory()
            .expect("Failed to get memory export");
        let mut store = caller.as_context_mut();

        // Deserialize parameters from linear memory
        let mut data = vec![0; len];
        memory.read(&store, ptr, &mut data).expect("Failed to read parameters from memory");
        let deserialized: P = from_read(data.as_slice()).expect("Failed to deserialize parameters");

        // Call function and serialize output
        let result = func.call(deserialized);
        let serialized = rmp_serde::to_vec_named(&result).expect("Failed to serialize return values");

        // Allocate and store in linear memory
        let ptr = store.data().alloc.unwrap().call(&mut store, serialized.len() as i32 + 4)
            .expect("Failed to allocate memory") as usize;
        memory.write(&mut store, ptr, &(serialized.len() as i32).to_le_bytes()).expect("Failed to write data return to memory");
        memory.write(&mut store, ptr + 4, serialized.as_slice()).expect("Failed to write data return to memory");

        ptr as i32
    }
}
