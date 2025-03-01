use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemImpl, Path};

use crate::utils::{DependentType, Namespace};

pub struct BindingAttributeInputs {
    body_verbatim: TokenStream,
    is_multi_binding: bool,
    ns: Namespace,
    trait_: Path,
    concrete_impl: DependentType,
}

impl BindingAttributeInputs {
    pub fn from_input(
        is_multi_binding: bool,
        attr_inputs: proc_macro::TokenStream,
        body_inputs: proc_macro::TokenStream,
    ) -> syn::Result<BindingAttributeInputs> {
        if !attr_inputs.is_empty() {
            let error = if is_multi_binding {
                "#[multi_binding] takes no arguments"
            } else {
                "#[binding] takes no arguments"
            };
            return Err(syn::Error::new_spanned(
                TokenStream::from(attr_inputs),
                error,
            ));
        }

        let item = syn::parse::<ItemImpl>(body_inputs.clone())?;
        let Some((_, trait_, _)) = item.trait_ else {
            let error = if is_multi_binding {
                "#[multi_binding] must be applied to a trait impl"
            } else {
                "#[binding] must be applied to a trait impl"
            };
            return Err(syn::Error::new_spanned(item, error));
        };
        let concrete_impl = DependentType::from_raw_type(&item.self_ty)?;
        let ns = Namespace::from_trait_impl(&trait_, &concrete_impl.inner);

        Ok(BindingAttributeInputs {
            body_verbatim: body_inputs.into(),
            is_multi_binding,
            ns,
            trait_,
            concrete_impl
        })
    }

    pub fn generate_code(self) -> proc_macro::TokenStream {
        let create_fn = self.get_create_fn();
        let binding_meta = self.get_binding_meta();
        let original = self.body_verbatim;

        quote! {
            #create_fn
            #binding_meta
            #original
        }.into()
    }

    fn get_create_fn(&self) -> TokenStream {
        let concrete_type = self.concrete_impl.as_stripped_type();
        let create_fn_name = self.ns.name_of_create_fn();
        let trait_ = &self.trait_;

        quote! {
            unsafe fn #create_fn_name(injector: &::injector::Injector) -> ::std::boxed::Box<dyn ::std::any::Any> {
                let concrete_type = injector.get();
                let static_concrete_type = unsafe {
                    // SAFETY: See safety docs in BindingMeta::create
                    ::std::mem::transmute::<&#concrete_type, &'static <#concrete_type as ::injector::Injectable>::Static>(concrete_type)
                };
                let trait_object: &dyn #trait_ = &*static_concrete_type;

                ::std::boxed::Box::new(trait_object)
            }
        }
    }

    fn get_binding_meta(&self) -> TokenStream {
        let inject_meta_fn = self.ns.name_of_inject_meta_fn();
        let create_fn_name = self.ns.name_of_create_fn();
        let trait_ = &self.trait_;
        let impl_type_id = self.concrete_impl.quote_type_id();
        let is_multi_binding = self.is_multi_binding;

        quote! {
            #[::injector::derive_api::distributed_slice(::injector::derive_api::BINDING_REGISTRY)]
            fn #inject_meta_fn() -> ::injector::derive_api::BindingMeta {
                ::injector::derive_api::BindingMeta {
                    trait_object: ::std::any::TypeId::of::<&'static dyn #trait_>(),
                    name: ::std::any::type_name::<dyn #trait_>(),
                    impl_type: #impl_type_id,
                    is_multi_binding: #is_multi_binding,
                    create: #create_fn_name,
                }
            }
        }
    }
}
