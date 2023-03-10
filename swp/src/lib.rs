#![feature(fn_traits)]

pub use swp_proc_macros::{swp, swp_extern};
pub use rmp_serde;
pub use rmpv::Value;
use serde::Serialize;

// https://radu-matei.com/blog/practical-guide-to-wasm-memory/#passing-arrays-to-rust-webassembly-modules
// Allocate memory into the module's linear memory
// and return the offset to the start of the block.
#[no_mangle]
pub fn alloc(len: usize) -> *mut u8 {
    // Create a new mutable buffer with capacity `len`
    let mut buf = Vec::with_capacity(len);
    // Take a mutable pointer to the buffer
    let ptr = buf.as_mut_ptr();
    // Take ownership of the memory block and
    //     ensure that its destructor is not
    //     called when the object goes out of scope
    //     at the end of the function
    std::mem::forget(buf);
    // Return the pointer so the runtime
    //     can write data at this offset
    return ptr;
}

pub fn to_value<T: Serialize>(data: &T) -> Value {
    rmp_serde::from_slice(
        &rmp_serde::to_vec_named(data).unwrap()
    ).unwrap()
}
