use std::mem;

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Attribute, Field, FnArg, GenericArgument, Path, PathArguments, TraitBoundModifier, Type,
    TypeParamBound, TypePath, TypeTraitObject, spanned::Spanned,
};

mod error_messages {
    pub const NEEDS_BORROW: &str = "Types supplied by the injector must be references";
    pub const SIMPLE_DEPS_ONLY: &str = "Only simple types can be injected at this time";
    pub const NO_RECEIVER: &str = "Constructor functions cannot take receiver parameters";
    pub const SIMPLE_TRAIT_BOUNDS_ONLY: &str =
        "Only simple trait bounds can be injected at this time";
}

pub enum DependentType {
    RegularType(TypePath),
    TraitObject(Path),
    CollectionOfTraitObjects(Path),
}

pub struct Namespace {
    inner: String,
    references: Span,
}

impl DependentType {
    pub fn from_field(field: &Field) -> syn::Result<Self> {
        if let Some(output) = Self::from_attributes(&field.attrs)? {
            Ok(output)
        } else {
            Self::from_reference_type(&field.ty)
        }
    }

    pub fn from_fn_arg(fn_arg: &FnArg) -> syn::Result<Self> {
        match fn_arg {
            FnArg::Typed(pat_type) => {
                if let Some(output) = Self::from_attributes(&pat_type.attrs)? {
                    Ok(output)
                } else {
                    Self::from_reference_type(&pat_type.ty)
                }
            }
            FnArg::Receiver(inner) => {
                Err(syn::Error::new_spanned(inner, error_messages::NO_RECEIVER))
            }
        }
    }

    pub fn from_raw_type(ty: &Type) -> syn::Result<Self> {
        match ty {
            Type::Path(inner) => Ok(DependentType::RegularType(inner.clone())),
            Type::TraitObject(trait_) => {
                Ok(DependentType::TraitObject(Self::from_trait_object(trait_)?))
            }
            other => Err(syn::Error::new_spanned(
                other,
                error_messages::SIMPLE_DEPS_ONLY,
            )),
        }
    }

    pub fn quote_get_call(&self) -> TokenStream {
        match self {
            DependentType::RegularType(_) => quote!(injector.get()),
            DependentType::TraitObject(_) => quote!(injector.get_trait_object()),
            DependentType::CollectionOfTraitObjects(_) => quote!(
                ::std::iter::FromIterator::from_iter(injector.get_all_trait_objects())
            ),
        }
    }

    pub fn quote_type_id(&self) -> impl ToTokens {
        match self {
            DependentType::RegularType(ty) => {
                let mut ty = ty.clone();
                strip_lifetimes(&mut ty.path);
                quote!(::std::any::TypeId::of::<#ty>())
            }
            DependentType::TraitObject(trait_)
            | DependentType::CollectionOfTraitObjects(trait_) => {
                let mut trait_ = trait_.clone();
                strip_lifetimes(&mut trait_);
                quote!(::std::any::TypeId::of::<&'static dyn #trait_>())
            }
        }
    }

    fn from_reference_type(ty: &Type) -> syn::Result<Self> {
        match ty {
            Type::Reference(referenced_type) => Self::from_raw_type(&referenced_type.elem),
            other => Err(syn::Error::new_spanned(other, error_messages::NEEDS_BORROW)),
        }
    }

    fn from_attributes(attrs: &[Attribute]) -> syn::Result<Option<Self>> {
        let attrs = attrs
            .iter()
            .filter(|attr| attr.path().is_ident("from_multi_binding"))
            .map(|attr| attr.parse_args::<TypeTraitObject>())
            .collect::<syn::Result<Vec<_>>>()?;

        let attr = match attrs.as_slice() {
            [] => return Ok(None),
            [single] => single,
            [_, second, ..] => {
                return Err(syn::Error::new_spanned(
                    second,
                    "Only one #[has_constructor] attribute is allowed",
                ));
            }
        };

        let output = Self::from_trait_object(attr)?;
        Ok(Some(DependentType::CollectionOfTraitObjects(output)))
    }

    fn from_trait_object(trait_: &TypeTraitObject) -> syn::Result<Path> {
        let trait_bounds = trait_
            .bounds
            .iter()
            .filter_map(|bound| match bound {
                TypeParamBound::Trait(trait_bound) => {
                    if let Some(lt) = &trait_bound.lifetimes {
                        Some(Err(syn::Error::new_spanned(
                            lt,
                            error_messages::SIMPLE_TRAIT_BOUNDS_ONLY,
                        )))
                    } else if let TraitBoundModifier::Maybe(question) = trait_bound.modifier {
                        Some(Err(syn::Error::new_spanned(
                            question,
                            error_messages::SIMPLE_TRAIT_BOUNDS_ONLY,
                        )))
                    } else {
                        Some(Ok(&trait_bound.path))
                    }
                }
                TypeParamBound::Lifetime(_) => None,
                TypeParamBound::PreciseCapture(inner) => Some(Err(syn::Error::new_spanned(
                    inner,
                    error_messages::SIMPLE_TRAIT_BOUNDS_ONLY,
                ))),
                TypeParamBound::Verbatim(inner) => Some(Err(syn::Error::new_spanned(
                    inner,
                    error_messages::SIMPLE_TRAIT_BOUNDS_ONLY,
                ))),
                other => Some(Err(syn::Error::new_spanned(
                    other,
                    error_messages::SIMPLE_TRAIT_BOUNDS_ONLY,
                ))),
            })
            .collect::<syn::Result<Vec<_>>>()?;

        match trait_bounds.as_slice() {
            [] => Err(syn::Error::new_spanned(
                trait_,
                error_messages::SIMPLE_TRAIT_BOUNDS_ONLY,
            )),
            [single] => Ok((*single).clone()),
            [_, second, ..] => Err(syn::Error::new_spanned(
                second,
                error_messages::SIMPLE_TRAIT_BOUNDS_ONLY,
            )),
        }
    }
}

impl Namespace {
    pub fn from_type_name(ident: &Ident) -> Self {
        let inner = ident
            .to_string()
            .from_case(Case::Pascal)
            .to_case(Case::Snake);
        let references = ident.span();
        Namespace { inner, references }
    }

    pub fn from_fn_name(ident: &Ident) -> Self {
        let inner = ident.to_string();
        let references = ident.span();
        Namespace { inner, references }
    }

    pub fn from_trait_impl(trait_: &Path, target: &DependentType) -> Self {
        let mut inner = String::new();
        let target = match target {
            DependentType::RegularType(path) => path.path.segments.iter(),
            DependentType::TraitObject(path) => path.segments.iter(),
            DependentType::CollectionOfTraitObjects(_) => unreachable!(),
        };
        for segment in trait_.segments.iter().chain(target) {
            if !inner.is_empty() {
                inner.push('_');
            }
            inner.push_str(
                &segment
                    .ident
                    .to_string()
                    .from_case(Case::Pascal)
                    .to_case(Case::Snake),
            );
        }

        let references = trait_.span();
        Namespace { inner, references }
    }

    pub fn name_of_create_fn(&self) -> Ident {
        Ident::new(
            &format!("__injector_create_fn_{}", self.inner),
            self.references,
        )
    }

    pub fn name_of_inject_meta_fn(&self) -> Ident {
        Ident::new(
            &format!("__injector_inject_meta_fn_{}", self.inner),
            self.references,
        )
    }
}

pub fn quote_inject_meta(
    type_name: impl ToTokens,
    ns: &Namespace,
    dependencies: impl Iterator<Item = syn::Result<DependentType>>,
) -> syn::Result<TokenStream> {
    let dependencies = dependencies.collect::<syn::Result<Vec<_>>>()?;
    let dependencies = dependencies.iter().map(|dep| dep.quote_type_id());
    let dependencies = quote!(::std::vec![#(#dependencies),*]);
    let create_fn_name = ns.name_of_create_fn();
    let inject_meta_fn_name = ns.name_of_inject_meta_fn();

    Ok(quote! {
        #[::injector::derive_api::linkme::distributed_slice(::injector::derive_api::INJECTION_REGISTRY)]
        #[linkme(crate = ::injector::derive_api::linkme)]
        fn #inject_meta_fn_name() -> ::injector::derive_api::InjectMeta {
            ::injector::derive_api::InjectMeta {
                this: ::std::any::TypeId::of::<#type_name>(),
                name: ::std::any::type_name::<#type_name>(),
                dependencies: #dependencies,
                create: #create_fn_name,
                is_multi_binding: false,
            }
        }
    })
}

pub fn strip_lifetimes(path: &mut Path) {
    for segment in &mut path.segments.iter_mut() {
        let PathArguments::AngleBracketed(generics) = &mut segment.arguments else {
            continue;
        };

        let old_args = mem::replace(&mut generics.args, Default::default());
        generics.args.extend(
            old_args
                .into_iter()
                .filter(|arg| !matches!(arg, GenericArgument::Lifetime(_))),
        );
    }
}
