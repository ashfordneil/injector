use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Fields, GenericParam, Generics, Meta};

use crate::utils::{self, DependentType, Namespace};

pub struct InjectableDeriveInputs {
    type_name: Ident,
    ns: Namespace,
    has_lifetime: bool,
    // If this is left as None, that means they have their own constructor elsewhere
    fields: Option<Fields>,
}

impl InjectableDeriveInputs {
    pub fn from_input(input: proc_macro::TokenStream) -> syn::Result<Self> {
        let raw_input: DeriveInput = syn::parse(input)?;

        let type_name = raw_input.ident.clone();
        let ns = Namespace::from_type_name(&type_name);
        let has_lifetime = Self::has_lifetime(&raw_input.generics)?;
        let fields = Self::get_fields(raw_input)?;

        Ok(InjectableDeriveInputs {
            type_name,
            ns,
            has_lifetime,
            fields,
        })
    }

    fn has_lifetime(input: &Generics) -> syn::Result<bool> {
        let mut has_lifetime = false;
        for param in input.params.iter() {
            let GenericParam::Lifetime(lifetime) = param else {
                return Err(syn::Error::new_spanned(
                    param,
                    "Injectable types are only allowed to be generic over a single lifetime parameter",
                ));
            };

            if has_lifetime {
                return Err(syn::Error::new_spanned(
                    lifetime,
                    "Injectable types are only allowed to have one lifetime",
                ));
            }
            has_lifetime = true;
        }

        Ok(has_lifetime)
    }

    fn get_fields(input: DeriveInput) -> syn::Result<Option<Fields>> {
        if Self::has_constructor_annotation(input.attrs.iter())? {
            return Ok(None);
        }

        let Data::Struct(data) = input.data else {
            return Err(syn::Error::new_spanned(
                input,
                "Injectable can only be derived for structs or types with a #[has_constructor] attribute.",
            ));
        };

        Ok(Some(data.fields))
    }

    fn has_constructor_annotation<'a>(
        attrs: impl Iterator<Item = &'a Attribute>,
    ) -> syn::Result<bool> {
        let attrs = attrs
            .filter(|attr| attr.path().is_ident("has_constructor"))
            .map(|attr| match &attr.meta {
                Meta::Path(inner) => Ok(inner),
                Meta::List(list) => Err(syn::Error::new_spanned(
                    &list.tokens,
                    "#[has_constructor] takes no arguments",
                )),
                Meta::NameValue(name_value) => Err(syn::Error::new_spanned(
                    &name_value.value,
                    "#[has_constructor] needs no value",
                )),
            })
            .collect::<syn::Result<Vec<_>>>()?;

        match attrs.as_slice() {
            [] => Ok(false),
            [_single] => Ok(true),
            [_, second, ..] => Err(syn::Error::new_spanned(
                second,
                "Only one #[has_constructor] attribute is allowed",
            )),
        }
    }

    pub fn derive(self) -> syn::Result<proc_macro::TokenStream> {
        let base_impl = self.get_base_impl();
        let static_impl = self.get_static_impl();
        let create_fn = self.get_create_fn()?;
        let create_meta = self.get_create_meta()?;

        Ok(quote! {
            #base_impl
            #static_impl
            #create_fn
            #create_meta
        }
        .into())
    }

    fn get_base_impl(&self) -> TokenStream {
        let static_type = self.static_self_type();
        let borrowed_type = self.borrowed_self_type();

        quote! {
            impl <'a> ::injector::Injectable<'a> for #borrowed_type {
                type Static = #static_type;

                unsafe fn upcast(self) -> Self::Static {
                    // SAFETY: see docs for upcast in the trait declaration. This is exactly what we
                    // are meant to do here.
                    unsafe { ::std::mem::transmute::<Self, Self::Static>(self) }
                }
            }
        }
    }

    fn get_static_impl(&self) -> TokenStream {
        let static_type = self.static_self_type();
        let borrowed_type = self.borrowed_self_type();

        quote! {
            impl ::injector::derive_api::InjectableStatic for #static_type {
                type Injectable<'a> = #borrowed_type;

                fn downcast(&self) -> &Self::Injectable<'_> {
                    self
                }
            }
        }
    }

    fn get_create_meta(&self) -> syn::Result<TokenStream> {
        let Some(fields) = &self.fields else {
            // If there's no fields, they will need to get their create_meta from the constructor
            return Ok(quote! {});
        };

        let deps = fields.iter().map(DependentType::from_field);
        utils::quote_inject_meta(&self.type_name, &self.ns, deps)
    }

    fn get_create_fn(&self) -> syn::Result<TokenStream> {
        let Some(fields) = &self.fields else {
            return Ok(quote!());
        };

        let type_name = &self.type_name;
        let constructed = match fields {
            Fields::Named(fields) => {
                let fields = fields
                    .named
                    .iter()
                    .map(|field| {
                        let dependency = DependentType::from_field(&field)?.quote_get_call();
                        let field_name = field.ident.as_ref().unwrap();
                        Ok(quote! { #field_name: #dependency })
                    })
                    .collect::<syn::Result<Vec<_>>>()?;
                quote! { #type_name { #(#fields),* } }
            }
            Fields::Unnamed(fields) => {
                let fields = fields
                    .unnamed
                    .iter()
                    .map(|field| DependentType::from_field(&field).map(|dep| dep.quote_get_call()))
                    .collect::<syn::Result<Vec<_>>>()?;
                quote! { #type_name(#(#fields),*) }
            }
            Fields::Unit => quote! { #type_name },
        };

        let create_fn_name = self.ns.name_of_create_fn();
        Ok(quote! {
            fn #create_fn_name(injector: &::injector::Injector) -> ::std::boxed::Box<dyn ::std::any::Any> {
                let constructed = #constructed;
                ::std::boxed::Box::new(unsafe {
                    <#type_name as ::injector::Injectable>::upcast(constructed)
                })
            }
        })
    }

    fn static_self_type(&self) -> TokenStream {
        let name = &self.type_name;
        if self.has_lifetime {
            quote!(#name <'static>)
        } else {
            quote!(#name)
        }
    }

    fn borrowed_self_type(&self) -> TokenStream {
        let name = &self.type_name;
        if self.has_lifetime {
            quote!(#name<'a>)
        } else {
            quote!(#name)
        }
    }
}
