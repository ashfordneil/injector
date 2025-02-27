use injector::{Injectable, Injector};

fn main() {
    let mut injector = Injector::new();
    injector.prebuild::<Everything<'static>>();
    let empty: &Empty = injector.get();
    let everything: &Everything = injector.get();
    assert_eq!(&raw const *empty, &raw const *everything.holds.empty);
    assert_eq!("hello, world", everything.has.data)
}

#[derive(Injectable)]
struct Empty {}

#[derive(Injectable)]
struct HoldsAnEmpty<'a> {
    empty: &'a Empty,
}

#[derive(Injectable)]
#[constructor(inject_has_data)]
struct NeedsCustomConstructor {
    data: String,
}

#[derive(Injectable)]
struct Everything<'a> {
    holds: &'a HoldsAnEmpty<'a>,
    has: &'a NeedsCustomConstructor,
}

fn inject_has_data(_: &Injector) -> NeedsCustomConstructor {
    NeedsCustomConstructor {
        data: "hello, world".to_string(),
    }
}
