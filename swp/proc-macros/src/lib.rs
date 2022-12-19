extern crate proc_macro;
use proc_macro::TokenStream;

use proc_macro2::Span;
use quote::quote;
use syn::{ItemFn, Ident, FnArg, Type};

#[proc_macro_attribute]
pub fn swp(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = proc_macro2::TokenStream::from(item);
    let mut ast: ItemFn = syn::parse2(input).unwrap();

    let name = ast.sig.ident;
    let internal_name = Ident::new(
        &format!("internal_{}", name),
        Span::call_site()
    );
    ast.sig.ident = internal_name.clone();

    let input_types: Vec<Type> = ast.sig.inputs.iter().filter_map(|arg| {
        match arg {
            FnArg::Receiver(_) => None,
            FnArg::Typed(x) => Some(*x.clone().ty)
        }
    }).collect();
    let input_type = quote! { (#(#input_types),*) };

    let expanded = quote! {
        #[no_mangle]
        pub unsafe extern "C" fn #name(ptr: *mut u8, len: usize) -> (*mut u8) {
            #ast

            use std::io::{Cursor, Write};

            // Read arguments from linear memory
            let data = Vec::from_raw_parts(ptr, len, len);
            // Deserialize arguments
            let i: #input_type = match swp::rmp_serde::from_slice(data.as_slice()) {
                Ok(i) => i,
                Err(_) => { panic!("RPC called with invalid arguments!") }
            };

            // Create buffer + cursor, start writing after length placeholder
            let mut buffer: Vec<u8> = vec![0,0,0,0];
            let mut cursor = Cursor::new(&mut buffer);
            cursor.set_position(4);
            // Serialize return values to buffer
            swp::rmp_serde::encode::write_named(
                &mut cursor,
                &std::ops::Fn::call(&#internal_name, i)
            ).unwrap();
            // Write length
            let len: u32 = cursor.position() as u32 - 4;
            cursor.set_position(0);
            cursor.write_all(&len.to_le_bytes());

            // Return pointer
            let pointer = buffer.as_mut_ptr();
            std::mem::forget(buffer);
            pointer
        }
    };

    println!("{}", expanded);
    TokenStream::from(expanded)
}
