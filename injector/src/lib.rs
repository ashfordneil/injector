#[doc(hidden)]
pub mod derive_api;
mod runtime;

pub use injector_derive::{Injectable, binding, constructor, multi_binding};
pub use runtime::Injector;

/// A type that the [`Injector`] can manage. This type should have a set of dependencies (which are
/// also [`Injectable`]), and a way to construct the type from those dependencies. Use the
/// accompanying derive macro rather than implementing this by hand, as there is a lot behind the
/// scenes.
pub trait Injectable<'a>
where
    Self: 'a,
{
    /// An equivalent version of `Self` which is not parameterised by any lifetimes (i.e. its
    /// `'static`).
    #[doc(hidden)]
    type Static: derive_api::InjectableStatic<Injectable<'a> = Self>;

    /// Convert an instance of this type, which is parameterised by a lifetime to ensure that it
    /// does not outlive its dependencies, into an instance of `Self::Static`, which does not have
    /// any lifetime information stored at the type level.
    ///
    /// # Safety
    /// This function takes `&'a Dependencies` in self and turns them into `&'static Dependencies`,
    /// and that is what makes it unsafe. To use it safely, ensure that you treat the returned
    /// object as if it was still bound by (and cannot outlive) `&'a`. There is a matching
    /// [`derive_api::InjectableStatic::downcast`] function that can return this object back to its
    /// original type to help with this.
    #[doc(hidden)]
    unsafe fn upcast(self) -> Self::Static;
}
