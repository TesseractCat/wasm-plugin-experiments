#![feature(fn_traits)]

use swp::*;
use serde::{Serialize, Deserialize};

#[swp_extern]
extern "C" {
    fn print(text: &str);
}

#[swp]
fn echo(text: &str) {
    print(text);
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
fn extract(person: Person) -> String {
    person.name
}
