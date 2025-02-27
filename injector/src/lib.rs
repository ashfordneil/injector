use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet, VecDeque},
};

use lazy_static::lazy_static;
pub use linkme::distributed_slice;

pub use injector_derive::Injectable;

pub trait Injectable<'a>
where
    Self: 'a,
{
    type Static: InjectableStatic<Injectable<'a> = Self>;

    unsafe fn upcast(self) -> Self::Static;
}

#[doc(hidden)]
pub trait InjectableStatic: Any {
    type Injectable<'a>: Injectable<'a, Static = Self>;

    fn downcast(&self) -> &Self::Injectable<'_>;
}

#[doc(hidden)]
pub struct InjectMeta {
    pub this: TypeId,
    pub dependencies: Vec<TypeId>,
    pub create: fn(&Injector) -> Box<dyn Any>,
    pub name: &'static str,
}

#[doc(hidden)]
#[distributed_slice]
pub static INJECTIONS: [fn() -> InjectMeta];

lazy_static! {
    static ref INJECTIONS_INDEX: HashMap<TypeId, InjectMeta> = INJECTIONS
        .iter()
        .map(|f| (*f)())
        .map(|v| (v.this, v))
        .collect();
}

pub struct Injector {
    items: Vec<Box<dyn Any>>,
    mapping: HashMap<TypeId, usize>,
}

impl Injector {
    pub fn new() -> Self {
        Injector {
            items: Vec::new(),
            mapping: HashMap::new(),
        }
    }

    /// Assuming the object has already been created
    pub fn get<'a, I: Injectable<'a>>(&'a self) -> &'a I {
        let type_id = TypeId::of::<I::Static>();
        let list_id = self.mapping.get(&type_id).unwrap();
        let static_item: &I::Static = self.items[*list_id].downcast_ref().unwrap();
        static_item.downcast()
    }

    /// Creates the object, then you can get it with [`Self::get`]
    pub fn prebuild<I: InjectableStatic>(&mut self) {
        let mut create_order = Vec::new();
        let mut visit_queue = VecDeque::new();
        let mut done = HashSet::new();
        let mut in_progress = HashSet::new();

        enum VisitType {
            BeforeChildren,
            AfterChildren,
        }

        let start = TypeId::of::<I>();
        visit_queue.push_back((start, VisitType::BeforeChildren));
        visit_queue.push_back((start, VisitType::AfterChildren));
        while let Some((to_visit, visit_type)) = visit_queue.pop_front() {
            if done.contains(&to_visit) {
                continue;
            }
            match visit_type {
                VisitType::BeforeChildren => {
                    let meta = INJECTIONS_INDEX.get(&to_visit).unwrap();
                    assert!(
                        in_progress.insert(to_visit),
                        "Loop detected for type {:?}",
                        meta.name
                    );
                    for dep in meta.dependencies.iter() {
                        if !done.contains(dep) {
                            visit_queue.push_front((*dep, VisitType::AfterChildren));
                            visit_queue.push_front((*dep, VisitType::BeforeChildren));
                        }
                    }
                }
                VisitType::AfterChildren => {
                    create_order.push(to_visit);
                    in_progress.remove(&to_visit);
                    done.insert(to_visit);
                }
            }
        }

        for item in create_order {
            let meta = INJECTIONS_INDEX.get(&item).unwrap();
            let output = (meta.create)(&self);
            self.items.push(output);
            self.mapping.insert(item, self.items.len() - 1);
        }
    }
}
