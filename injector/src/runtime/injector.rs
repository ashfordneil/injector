use std::{any::TypeId, collections::HashMap};

use super::{builder::InjectorBuilder, unsafe_storage::UnsafeStore};
use crate::{
    Injectable,
    derive_api::{InjectMeta, InjectableStatic},
};

/// The runtime that manages our injections. You should only need a single [`Injector`], that is
/// created at the top level of your program, and then you can call [`Injector::get`] on it as
/// needed.
///
/// The injector does all creations upfront. Once it has been created, any call to [`Injector::get`]
/// is just a map lookup.
pub struct Injector {
    items: UnsafeStore,
    index: HashMap<TypeId, usize>,
}

impl Injector {
    /// Every type which derives [`crate::Injectable`] gets added to a global registry. This builds
    /// all of those types, and returns an Injector that can supply any of them through [`Self::get`].
    pub fn new() -> Self {
        InjectorBuilder::new(Injector {
            items: UnsafeStore::new(),
            index: HashMap::new(),
        })
        .build_the_world()
    }

    /// Fetch an item from the injector cache. This will panic if for some reason the object does
    /// not exist.
    pub fn get<'a, I: Injectable<'a>>(&'a self) -> &'a I {
        let Some(&position) = self.index.get(&TypeId::of::<I::Static>()) else {
            panic!(
                "Unable to get an instance of {} from the injector.",
                std::any::type_name::<I::Static>()
            )
        };

        let static_item: &I::Static = UnsafeStore::get(&self.items, position)
            .unwrap() // any usize in the `index` has to map to an item in the UnsafeStore
            .downcast_ref()
            .unwrap(); // We check that the `dyn Any`s match up with what they say they do on insert

        // SAFETY: This static item is super unsafe, because the type system does not know that it
        // cannot outlive the injector. Make sure we downcast it before sending it anywhere
        static_item.downcast()
    }

    pub(super) fn build_and_store(&mut self, metadata: &InjectMeta) {
        let static_item = unsafe {
            // SAFETY: The item returned by metadata.create is super unsafe, because the type system
            // does not know that it cannot outlive its dependencies.
            //
            // 1. Within the injector: the static item cannot outlive anything that was already
            //    in the injector when it was created. We put it in the UnsafeStore immediately, and
            //    it will take care of that for us.
            // 2. Outside the injector: the static item cannot outlive the injector. When we get
            //    it out of the UnsafeStore, we must downcast it before returning it anywhere.
            (metadata.create)(&self)
        };

        assert_eq!(
            static_item.as_ref().type_id(),
            metadata.this,
            "Incorrect type returned by the Injectable s constructor for {}",
            metadata.name
        );

        let position = UnsafeStore::push(&mut self.items, static_item);
        self.index.insert(metadata.this, position);
    }
}
