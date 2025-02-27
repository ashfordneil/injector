use std::borrow::Cow;

use convert_case::{Case, Casing};
use proc_macro2::Ident;
use syn::{
    Attribute, Data, DeriveInput, ExprPath, Fields, GenericArgument, GenericParam, Generics,
    Lifetime, PathArguments, Type, TypePath,
};

pub struct InjectableDeriveInputs {
    name: Ident,
    lifetime: Option<Lifetime>,
    constructor_info: InjectableDeriveConstructorInfo,
}

enum InjectableDeriveConstructorInfo {
    CustomConstructor { name: ExprPath },
    PerField { fields: Fields },
}

impl InjectableDeriveInputs {
    pub fn from_input(input: proc_macro::TokenStream) -> syn::Result<Self> {
        let raw_input: DeriveInput = syn::parse(input)?;

        let name = raw_input.ident.clone();
        let lifetime = Self::get_lifetime(&raw_input.generics)?;
        let constructor_info = InjectableDeriveConstructorInfo::from_input(raw_input)?;

        Ok(InjectableDeriveInputs {
            name,
            lifetime,
            constructor_info,
        })
    }

    fn get_lifetime(input: &Generics) -> syn::Result<Option<Lifetime>> {
        let mut output = None;
        for param in input.params.iter() {
            let GenericParam::Lifetime(lifetime) = param else {
                return Err(syn::Error::new_spanned(
                    param,
                    "Injectable types are only allowed to be generic over a single lifetime parameter",
                ));
            };

            if output.is_some() {
                return Err(syn::Error::new_spanned(
                    lifetime,
                    "Injectable types are only allowed to have one lifetime",
                ));
            }
            output = Some(lifetime.lifetime.clone());
        }

        Ok(output)
    }

    pub fn derive(self) -> syn::Result<proc_macro2::TokenStream> {
        let mod_name = self.mod_name();
        let base_impl = self.get_base_impl();
        let static_impl = self.get_static_impl();
        let create_fn = self.get_create_fn();
        let create_meta = self.get_create_meta()?;

        Ok(quote::quote! {
            mod #mod_name {
                #base_impl
                #static_impl
                #create_fn
                #create_meta
            }
        })
    }

    fn mod_name(&self) -> Ident {
        let snake_cased = self
            .name
            .to_string()
            .from_case(Case::Pascal)
            .to_case(Case::Snake);

        Ident::new(
            &format!("__injector_injectable_impl_for_{snake_cased}"),
            self.name.span(),
        )
    }

    fn get_base_impl(&self) -> proc_macro2::TokenStream {
        let static_type = self.static_self_type();
        let borrowed_type = self.borrowed_self_type();

        quote::quote! {
            impl <'a> ::injector::Injectable<'a> for #borrowed_type {
                type Static = #static_type;

                unsafe fn upcast(self) -> Self::Static {
                    ::std::mem::transmute::<Self, Self::Static>(self)
                }
            }
        }
    }

    fn get_static_impl(&self) -> proc_macro2::TokenStream {
        let static_type = self.static_self_type();
        let borrowed_type = self.borrowed_self_type();

        quote::quote! {
            impl ::injector::InjectableStatic for #static_type {
                type Injectable<'a> = #borrowed_type;

                fn downcast(&self) -> &Self::Injectable<'_> {
                    self
                }
            }
        }
    }

    fn get_create_meta(&self) -> syn::Result<proc_macro2::TokenStream> {
        let static_type = self.static_self_type();
        let deps = self.get_deps()?;

        Ok(quote::quote! {
            #[::injector::distributed_slice(::injector::INJECTIONS)]
            fn create_meta() -> ::injector::InjectMeta {
                ::injector::InjectMeta {
                    this: ::std::any::TypeId::of::<#static_type>(),
                    dependencies: #deps,
                    create: create,
                    name: ::std::any::type_name::<#static_type>(),
                }
            }
        })
    }

    fn get_create_fn(&self) -> proc_macro2::TokenStream {
        let name = &self.name;
        let constructed = match &self.constructor_info {
            InjectableDeriveConstructorInfo::CustomConstructor { name } => {
                quote::quote! { (super::#name)(injector) }
            }
            InjectableDeriveConstructorInfo::PerField { fields } => match fields {
                Fields::Named(fields) => {
                    let fields = fields.named.iter().map(|field| {
                        let name = field.ident.as_ref().unwrap();
                        quote::quote! { #name: injector.get() }
                    });
                    quote::quote! { super::#name { #(#fields),* } }
                }
                Fields::Unnamed(fields) => {
                    let fields = fields
                        .unnamed
                        .iter()
                        .map(|_| quote::quote! { injector.get() });
                    quote::quote! { super::#name(#(#fields),*) }
                }
                Fields::Unit => quote::quote! { super::#name },
            },
        };

        quote::quote! {
            fn create(injector: &::injector::Injector) -> ::std::boxed::Box<dyn ::std::any::Any> {
                let constructed = #constructed;
                Box::new(unsafe {
                    <super::#name as ::injector::Injectable>::upcast(constructed)
                })
            }
        }
    }

    fn get_deps(&self) -> syn::Result<proc_macro2::TokenStream> {
        let InjectableDeriveConstructorInfo::PerField { fields } = &self.constructor_info else {
            return Ok(quote::quote! { ::std::vec![] });
        };

        let deps = fields.iter().map(|field| &field.ty)
            .map(|ty| match ty {
                Type::Reference(reference) => Ok(&reference.elem),
                _ => Err(syn::Error::new_spanned(ty, "Unable to inject this type automatically. Right now only references are supported")),
            })
            .map(|ref_ty| match &**ref_ty? {
                Type::Path(type_path) => {
                    let ty = if let Some(known_lifetime) = &self.lifetime {
                        Cow::Owned(make_type_static(type_path, known_lifetime))
                    } else {
                        Cow::Borrowed(type_path)
                    };
                    Ok(quote::quote!(::std::any::TypeId::of::<super::#ty>()))
                }
                other => Err(syn::Error::new_spanned(other, "Only plain types can be injected at this time"))
            }).collect::<syn::Result<Vec<_>>>()?;

        Ok(quote::quote! {
            ::std::vec![ #(#deps),* ]
        })
    }

    fn static_self_type(&self) -> proc_macro2::TokenStream {
        let name = &self.name;
        if self.lifetime.is_some() {
            quote::quote!(super::#name <'static>)
        } else {
            quote::quote!(super::#name)
        }
    }

    fn borrowed_self_type(&self) -> proc_macro2::TokenStream {
        let name = &self.name;
        if self.lifetime.is_some() {
            quote::quote!(super::#name<'a>)
        } else {
            quote::quote!(super::#name)
        }
    }
}

impl InjectableDeriveConstructorInfo {
    fn from_input(input: DeriveInput) -> syn::Result<Self> {
        if let Some(constructor) = Self::get_constructor_expression(input.attrs.iter())? {
            return Ok(InjectableDeriveConstructorInfo::CustomConstructor { name: constructor });
        }

        let Data::Struct(data) = input.data else {
            return Err(syn::Error::new_spanned(
                input,
                "Injectable can only be derived for structs or types with a #[constructor] attribute.",
            ));
        };

        Ok(InjectableDeriveConstructorInfo::PerField {
            fields: data.fields,
        })
    }

    fn get_constructor_expression<'a>(
        attrs: impl Iterator<Item = &'a Attribute>,
    ) -> syn::Result<Option<ExprPath>> {
        let attrs = attrs
            .filter(|attr| attr.path().is_ident("constructor"))
            .map(|attr| attr.parse_args::<ExprPath>())
            .collect::<syn::Result<Vec<_>>>()?;

        match attrs.as_slice() {
            [] => Ok(None),
            [single] => Ok(Some(single.clone())),
            [_, second, ..] => Err(syn::Error::new_spanned(
                second,
                "Only one #[constructor] attribute is allowed",
            )),
        }
    }



}

fn make_type_static(ty: &TypePath, lifetime: &Lifetime) -> TypePath {
    let mut output = ty.clone();
    for segment in output.path.segments.iter_mut() {
        let PathArguments::AngleBracketed(generics) = &mut segment.arguments else {
            continue;
        };

        for mut arg in generics.args.iter_mut() {
            if let GenericArgument::Lifetime(lt) = &mut arg {
                if lt == lifetime {
                    *lt = Lifetime::new("'static", lt.span());
                }
            }
        }
    }

    output
}
