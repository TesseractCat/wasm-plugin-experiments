#![feature(fn_traits)]

use swp::*;
use serde::{Serialize, Deserialize};

#[swp_extern]
extern "C" {
    fn print(text: &str, num: i32);
}

#[swp]
fn echo(text: &str) {
    print(text, 42);
}

#[derive(Serialize, Deserialize)]
struct Person {
    name: String,
    age: u32,
    cool: bool
}

#[swp]
fn add(a: &str, b: &str) -> String {
    String::from(a) + b
}

#[swp]
fn extract(person: Person) -> bool {
    person.cool
}
