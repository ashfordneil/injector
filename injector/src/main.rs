use std::env;

use injector::{constructor, Injectable, Injector};

fn main() {
    let injector = Injector::new();
    let everything: &Everything = injector.get();

    assert_eq!(&raw const *everything.simple, &raw const *everything.type_with_constructor.simple);
    assert_eq!(&raw const *everything.simple, &raw const *everything.basic_type.simple);
    println!("Hello, {}", everything.type_with_constructor.custom_field);
}

#[derive(Injectable)]
struct SimplestObject {}

#[derive(Injectable)]
struct BasicType<'a> {
    simple: &'a SimplestObject,
}

#[derive(Injectable)]
#[has_constructor]
struct TypeWithConstructor<'a> {
    simple: &'a SimplestObject,
    custom_field: String,
}

#[constructor]
fn build_type_with_constructor(simple: &SimplestObject) -> TypeWithConstructor {
    let custom_field = env::var("USER").unwrap_or_else(|_| String::new());
    TypeWithConstructor { simple, custom_field }
}

#[derive(Injectable)]
struct Everything<'a> {
    simple: &'a SimplestObject,
    basic_type: &'a BasicType<'a>,
    type_with_constructor: &'a TypeWithConstructor<'a>,
}
