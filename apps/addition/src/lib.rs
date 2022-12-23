#![feature(fn_traits)]

use std::collections::HashMap;

use swp::*;
use serde::{Serialize, Deserialize};

#[swp_extern]
extern "C" {
    fn print(text: &str);
    fn set_output(output: &str, data: Value);
}
fn set_display(data: Node) {
    set_output("display", to_value(&data));
}

#[swp]
fn echo(text: &str) {
    print(text);
}

#[derive(Serialize, Deserialize)]
struct Node {
    name: &'static str,
    attributes: HashMap<&'static str, String>,
    children: Vec<Node>,
}
impl Node {
    fn new(name: &'static str, attributes: HashMap<&'static str, String>, children: Vec<Node>) -> Self {
        Self {
            name, attributes, children
        }
    }
    fn leaf(name: &'static str, attributes: HashMap<&'static str, String>) -> Self {
        Self::new(
            name, attributes, vec![]
        )
    }
}

#[swp]
fn update() {
    set_display(Node::new(
        "root", HashMap::new(),
        vec![
            Node::leaf(
                "text", HashMap::new()
            )
        ]
    ));
}
