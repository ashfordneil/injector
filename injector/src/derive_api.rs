//! APIs that we need to make things work, but that we would really rather users of the crate not
//! know about. These APIs are all public, so that the derive macro can implement them, but they
//! should not be treated as visible **or stable**.

use std::any::{Any, TypeId};

pub use linkme::distributed_slice;

use crate::{Injectable, Injector};

/// A companion trait to [`Injectable`]. If you implement `Injectable<'a>` for `YourType<'a>`, then
/// you should implement `InjectableStatic` for `YourType<'static>`. Having a version of the type
/// that is not lifetime dependent is needed so that we can interface with basically anything from
/// [`std::any`], which we use heavily across this crate.
pub trait InjectableStatic: Any {
    /// The parameterised version of this type, with variable lifetimes.
    type Injectable<'a>: Injectable<'a, Static = Self>;

    /// Convert the lifetimes from `'static` down to a finite, borrowed, lifetime. An implementation
    /// of this should always just return self.
    fn downcast(&self) -> &Self::Injectable<'_>;
}

/// Runtime metadata about a type that the injector needs.
pub struct InjectMeta {
    /// The type ID of the [`InjectableStatic`] version of the type we are injecting.
    pub this: TypeId,

    /// The name of the type we are injecting.
    pub name: &'static str,

    // The type IDs of the [`InjectableStatic`] versions of the types we require for construction.
    pub dependencies: Vec<TypeId>,

    /// A function which creates our type. The injector is provided so that we are able to call
    /// [`Injector::get`] within this method. The injector runtime will ensure that dependencies are
    /// created before their dependents.
    ///
    /// # Safety
    /// In an ideal world, we would have `for<'a> fn(&'a Injector) -> dyn Injectable<'a>`. To work
    /// with dynamic types at runtime in rust, we unfortunately must upcast that `Injectable<'a>`
    /// into its equivalent `InjectableStatic`, which can then be boxed into a `dyn Any`.
    ///
    /// This is unsafe because it treats the `&'a Injector` borrow as `'static` to do that upcast.
    /// To implement this function safely, ensure that the only unsafety you have is an
    /// [`Injectable::upcast`] call.
    /// To call this function safely, we ensure:
    /// - The returned value of this function is stored inside the `Injector`, in a private field
    /// - Any time we use this value, we first call [`InjectableStatic::downcast`] and restore the
    ///     lifetime parameter to ensure that it does not outlive the injector that it borrowed from
    ///     to create it.
    /// - When dropping the `Injector`, we drop fields in the reverse order that they were created
    ///     in, so any references stored inside this value (which are to fields that were inside the
    ///     injector when this value was created) are still valid when [`std::ops::Drop::drop`] is called.
    pub create: unsafe fn(&Injector) -> Box<dyn Any>,
}

/// Runtime metadata for all the types that we want to inject, aggregated into one spot by the
/// linker. For more info, see the [`linkme`] crate.
#[distributed_slice]
pub static INJECTION_REGISTRY: [fn() -> InjectMeta];