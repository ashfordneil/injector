use std::{
    any::{Any, TypeId},
    mem,
};

use injector::{INJECTIONS, InjectMeta, Injectable, InjectableStatic, Injector, distributed_slice};
fn main() {
    let mut injector = Injector::new();
    injector.prebuild::<Everything<'static>>();
    let empty: &Empty = injector.get();
    let everything: &Everything = injector.get();
    assert_eq!(&raw const *empty, &raw const *everything.holds.empty);
    assert_eq!("hello, world", everything.has.data)
}

struct Empty {}

struct HoldsAnEmpty<'a> {
    empty: &'a Empty,
}

struct HasData {
    data: String,
}

struct Everything<'a> {
    holds: &'a HoldsAnEmpty<'a>,
    has: &'a HasData,
}

fn inject_empty(_: &Injector) -> Box<dyn Any> {
    Box::new(Empty {})
}

fn inject_holds_an_empty(injector: &Injector) -> Box<dyn Any> {
    let first = injector.get();
    Box::new(unsafe { HoldsAnEmpty { empty: first }.upcast() })
}

fn inject_has_data(_: &Injector) -> Box<dyn Any> {
    Box::new(HasData {
        data: "hello, world".to_string(),
    })
}

fn inject_everything(injector: &Injector) -> Box<dyn Any> {
    let holds = injector.get();
    let has = injector.get();
    Box::new(unsafe { Everything { holds, has }.upcast() })
}

impl<'a> Injectable<'a> for Empty {
    type Static = Self;

    unsafe fn upcast(self) -> Self::Static {
        self
    }
}

impl<'a> Injectable<'a> for HoldsAnEmpty<'a> {
    type Static = HoldsAnEmpty<'static>;

    unsafe fn upcast(self) -> Self::Static {
        let HoldsAnEmpty { empty: first } = self;

        let first = unsafe { mem::transmute(first) };
        HoldsAnEmpty { empty: first }
    }
}

impl<'a> Injectable<'a> for HasData {
    type Static = HasData;

    unsafe fn upcast(self) -> Self::Static {
        self
    }
}

impl<'a> Injectable<'a> for Everything<'a> {
    type Static = Everything<'static>;

    unsafe fn upcast(self) -> Self::Static {
        let Everything { holds, has } = self;
        let holds = unsafe { mem::transmute(holds) };
        let has = unsafe { mem::transmute(has) };
        Everything { holds, has }
    }
}

impl InjectableStatic for Empty {
    type Injectable<'a> = Self;

    fn downcast<'a>(&'a self) -> &'a Self::Injectable<'a> {
        self
    }
}

impl InjectableStatic for HoldsAnEmpty<'static> {
    type Injectable<'a> = HoldsAnEmpty<'a>;

    fn downcast<'a>(&'a self) -> &'a Self::Injectable<'a> {
        self
    }
}

impl InjectableStatic for HasData {
    type Injectable<'a> = HasData;

    fn downcast<'a>(&'a self) -> &'a Self::Injectable<'a> {
        self
    }
}

impl InjectableStatic for Everything<'static> {
    type Injectable<'a> = Everything<'a>;

    fn downcast<'a>(&'a self) -> &'a Self::Injectable<'a> {
        self
    }
}

#[distributed_slice(INJECTIONS)]
fn empty_injector() -> InjectMeta {
    InjectMeta {
        this: TypeId::of::<Empty>(),
        dependencies: vec![],
        create: inject_empty,
        name: std::any::type_name::<Empty>(),
    }
}

#[distributed_slice(INJECTIONS)]
fn holds_an_empty_injector() -> InjectMeta {
    InjectMeta {
        this: TypeId::of::<HoldsAnEmpty<'static>>(),
        dependencies: vec![TypeId::of::<Empty>()],
        create: inject_holds_an_empty,
        name: std::any::type_name::<HoldsAnEmpty<'static>>(),
    }
}

#[distributed_slice(INJECTIONS)]
fn has_data_injector() -> InjectMeta {
    InjectMeta {
        this: TypeId::of::<HasData>(),
        dependencies: vec![],
        create: inject_has_data,
        name: std::any::type_name::<HasData>(),
    }
}

#[distributed_slice(INJECTIONS)]
fn everything_injector() -> InjectMeta {
    InjectMeta {
        this: TypeId::of::<Everything<'static>>(),
        dependencies: vec![
            TypeId::of::<HoldsAnEmpty<'static>>(),
            TypeId::of::<HasData>(),
        ],
        create: inject_everything,
        name: std::any::type_name::<Everything<'static>>(),
    }
}
