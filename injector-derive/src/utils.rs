use std::mem;

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Field, FnArg, GenericArgument, PathArguments, Type, TypePath};

mod error_messages {
    pub const NEEDS_BORROW: &str = "Types supplied by the injector must be references";
    pub const SIMPLE_DEPS_ONLY: &str = "Only simple types can be injected at this time";
    pub const NO_RECEIVER: &str = "Constructor functions cannot take receiver parameters";
}

pub struct DependentType {
    inner: TypePath
}

pub struct Namespace {
    inner: String,
    references: Span,
}

impl DependentType {
    pub fn from_field(field: &Field) -> syn::Result<Self> {
        Self::from_type(&field.ty)
    }

    pub fn from_fn_arg(fn_arg: &FnArg) -> syn::Result<Self> {
        match fn_arg {
            FnArg::Typed(pat_type) => Self::from_type(&pat_type.ty),
            FnArg::Receiver(inner) => Err(syn::Error::new_spanned(inner, error_messages::NO_RECEIVER)),
        }

    }

    fn quote_dependency_vec(dependencies: impl Iterator<Item = syn::Result<Self>>) -> syn::Result<TokenStream> {
        let dependencies = dependencies.collect::<syn::Result<Vec<_>>>()?;
        let quoted_type_ids = dependencies.into_iter().map(|ty| {
            let stripped = strip_lifetimes(&ty.inner);
            quote!(::std::any::TypeId::of::<#stripped>())
        });
        Ok(quote!(::std::vec![#(#quoted_type_ids),*]))
    }

    fn from_type(ty: &Type) -> syn::Result<Self> {
        match ty {
            Type::Reference(referenced_type) => {
                match &*referenced_type.elem {
                    Type::Path(inner) => Ok(DependentType { inner: inner.clone() }),
                    other => Err(syn::Error::new_spanned(other, error_messages::SIMPLE_DEPS_ONLY)),
                }
            }
            other => Err(syn::Error::new_spanned(other, error_messages::NEEDS_BORROW)),
        }
    }
}


impl Namespace {
    pub fn from_type_name(ident: &Ident) -> Self {
        let inner = ident.to_string().from_case(Case::Pascal).to_case(Case::Snake);
        let references = ident.span();
        Namespace { inner, references }
    }

    pub fn from_fn_name(ident: &Ident) -> Self {
        let inner = ident.to_string();
        let references = ident.span();
        Namespace { inner, references }
    }

    pub fn name_of_create_fn(&self) -> Ident {
        Ident::new(&format!("__injector_create_fn_{}", self.inner), self.references)
    }

    pub fn name_of_inject_meta_fn(&self) -> Ident {
        Ident::new(&format!("__injector_inject_meta_fn_{}", self.inner), self.references)
    }
}

pub fn quote_inject_meta(type_name: impl ToTokens, ns: &Namespace, dependencies: impl Iterator<Item = syn::Result<DependentType>>) -> syn::Result<TokenStream> {
    let dependencies = DependentType::quote_dependency_vec(dependencies)?;
    let create_fn_name = ns.name_of_create_fn();
    let inject_meta_fn_name = ns.name_of_inject_meta_fn();

    Ok(quote! {
        #[::injector::derive_api::distributed_slice(::injector::derive_api::INJECTION_REGISTRY)]
        fn #inject_meta_fn_name() -> ::injector::derive_api::InjectMeta {
            ::injector::derive_api::InjectMeta {
                this: ::std::any::TypeId::of::<#type_name>(),
                name: ::std::any::type_name::<#type_name>(),
                dependencies: #dependencies,
                create: #create_fn_name,
            }
        }
    })
}

pub fn strip_lifetimes(ty: &TypePath) -> TypePath {
    let mut output = ty.clone();
    for segment in output.path.segments.iter_mut() {
        let PathArguments::AngleBracketed(generics) = &mut segment.arguments else {
            continue;
        };

        let old_args = mem::replace(&mut generics.args, Default::default());
        generics.args.extend(old_args.into_iter().filter(|arg| !matches!(arg, GenericArgument::Lifetime(_))));
    }

    output
}