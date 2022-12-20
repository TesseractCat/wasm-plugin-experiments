extern crate proc_macro;
use proc_macro::TokenStream;

use proc_macro2::Span;
use quote::quote;
use syn::{ItemFn, Ident, FnArg, Type, PatType, Pat, ItemForeignMod, ForeignItem};

#[proc_macro_attribute]
pub fn swp(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = proc_macro2::TokenStream::from(item);
    let ast: ItemFn = syn::parse2(input).unwrap();

    let name = ast.sig.ident.clone();
    let internal_name = Ident::new(
        &format!("client_{}", name),
        Span::call_site()
    );

    let input_types: Vec<Type> = ast.sig.inputs.iter().filter_map(|arg| {
        match arg {
            FnArg::Receiver(_) => None,
            FnArg::Typed(x) => Some(*x.clone().ty)
        }
    }).collect();
    let input_type = quote! { (#(#input_types),*,) };

    let expanded = quote! {
        #ast

        #[no_mangle]
        pub unsafe extern "C" fn #internal_name(ptr: *mut u8, len: usize) -> (*mut u8) {
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
                &std::ops::Fn::call(&#name, i)
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

    //println!("{}", expanded);
    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn swp_extern(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = proc_macro2::TokenStream::from(item.clone());
    let mut ast: ItemForeignMod = syn::parse2(input).unwrap();
    let mut bindings: Vec<ItemFn> = Vec::new();

    for item in ast.items.iter_mut() {
        match item {
            ForeignItem::Fn(x) => {
                println!("{:?}", x.sig.ident);

                let name = x.sig.ident.clone();
                let inputs = x.sig.inputs.clone();
                let input_types: Vec<Type> = inputs.iter().filter_map(|x| match x {
                    FnArg::Typed(PatType { ty, .. }) => Some(ty.as_ref().clone()),
                    _ => None
                }).collect();
                let input_names: Vec<Ident> = inputs.iter().filter_map(|x| match x {
                    FnArg::Typed(PatType { pat, .. }) => match pat.as_ref() {
                        Pat::Ident(id) => Some(id.ident.clone()),
                        _ => None
                    },
                    _ => None
                }).collect();
                let output = x.sig.output.clone();

                let host_name = Ident::new(&format!("host_{}", x.sig.ident), Span::call_site());
                bindings.push(syn::parse2(quote! {
                    fn #name(#inputs) #output {
                        let input_tuple: (#(#input_types),*,) = (#(#input_names),*,);
                        let mut serialized: Vec<u8> = swp::rmp_serde::to_vec(&input_tuple).unwrap();

                        let data = unsafe {
                            let ptr = #host_name(serialized.as_mut_ptr(), serialized.len());
                            // Read response from linear memory
                            let len = *(ptr as *const i32) as usize;
                            Vec::from_raw_parts(ptr.offset(4), len, len)
                        };

                        // Deserialize arguments
                        match swp::rmp_serde::from_slice(data.as_slice()) {
                            Ok(i) => i,
                            Err(_) => { panic!("RPC responded with invalid data!") }
                        }
                    }
                }).unwrap());

                x.sig.ident = host_name;
                x.sig.inputs.clear();
                x.sig.inputs.push(syn::parse2(quote! { ptr: *mut u8 }).unwrap());
                x.sig.inputs.push(syn::parse2(quote! { len: usize }).unwrap());
                x.sig.output = syn::parse2(quote! { -> *mut u8 }).unwrap();
            },
            _ => panic!("Encountered non-function in #[swp_extern] block")
        }
    }

    let expanded = quote! {
        #ast
        #(#bindings)*
    };
    println!("{}", expanded);

    TokenStream::from(expanded)
}
