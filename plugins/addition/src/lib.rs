#![feature(fn_traits)]

use swp::*;

#[swp]
fn add(a: &str, b: &str) -> String {
    String::from(a) + b
}

#[swp]
fn duck(a: i32, b: i32) {
    a + b;
}
