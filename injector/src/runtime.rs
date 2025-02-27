use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet, VecDeque},
};

use lazy_static::lazy_static;

use crate::{
    Injectable,
    derive_api::{INJECTION_REGISTRY, InjectMeta, InjectableStatic},
};

lazy_static! {
    static ref INJECTIONS_INDEX: HashMap<TypeId, InjectMeta> = INJECTION_REGISTRY
        .iter()
        .map(|f| (*f)())
        .map(|v| (v.this, v))
        .collect();
}

/// The runtime that manages our injections. You should only need a single [`Injector`], that is
/// created at the top level of your program, and then you can call [`Injector::get`] as needed.
pub struct Injector {
    items: Vec<Box<dyn Any>>,
    mapping: HashMap<TypeId, usize>,
}

impl Injector {
    /// Create a new injector.
    pub fn new() -> Self {
        Injector {
            items: Vec::new(),
            mapping: HashMap::new(),
        }
    }

    /// Get an object from the injector. This just loads the object from the cache, no work is done
    /// to create types here.
    pub fn get<'a, I: Injectable<'a>>(&'a self) -> &'a I {
        let type_id = TypeId::of::<I::Static>();
        let list_index = self
            .mapping
            .get(&type_id)
            .expect("Attempt to get a type which is not owned by the injector");
        // SAFETY: need to make sure we call downcast before sending this super unsafe static type
        // anywhere
        let static_item: &I::Static = self.items[*list_index].downcast_ref().unwrap();
        static_item.downcast()
    }

    /// Creates the object specified by `I`, and any of its dependencies, to populate the cache so
    /// that [`Self::get`] works.
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
            let output = unsafe { (meta.create)(&self) };
            self.items.push(output);
            self.mapping.insert(item, self.items.len() - 1);
        }
    }
}
