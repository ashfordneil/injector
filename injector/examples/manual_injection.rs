use injector::{Injectable, Injector};
use injector_derive::constructor;

fn main() {
    let manually = ManuallyInjectedValue { data: "Hello, world".to_string() };
    let injector = Injector::builder()
        .inject_value(manually)
        .build_the_world();
    let everything = injector.get::<Everything>();
    println!("{}", everything.manually.data);
}

#[derive(Injectable)]
#[has_constructor] // Calling inject_value acts as an equivalent to a constructor.
struct ManuallyInjectedValue {
    data: String,
}

#[derive(Injectable)]
#[has_constructor]
struct Everything<'a> {
    manually: &'a ManuallyInjectedValue,
}

#[constructor]
fn make_everything(manually: &ManuallyInjectedValue) -> Everything {
    Everything { manually }
}
