use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{FnArg, ItemFn, ReturnType, Type, TypePath};

use crate::utils::{self, DependentType, Namespace, strip_lifetimes};

pub struct ConstructorAttributeInputs {
    body_verbatim: TokenStream,
    constructor_name: Ident,
    ns: Namespace,
    output_type: TypePath,
    inputs: Vec<FnArg>,
}

impl ConstructorAttributeInputs {
    pub fn from_input(
        attr_inputs: proc_macro::TokenStream,
        body_inputs: proc_macro::TokenStream,
    ) -> syn::Result<Self> {
        if !attr_inputs.is_empty() {
            return Err(syn::Error::new_spanned(
                TokenStream::from(attr_inputs),
                "#[constructor] takes no arguments",
            ));
        }
        let item = syn::parse::<ItemFn>(body_inputs.clone())?;

        let constructor_name = item.sig.ident;
        let ns = Namespace::from_fn_name(&constructor_name);
        let output_type = Self::get_output_type(item.sig.output)?;
        let inputs = item.sig.inputs.into_iter().collect();

        Ok(ConstructorAttributeInputs {
            body_verbatim: body_inputs.into(),
            constructor_name,
            ns,
            output_type,
            inputs,
        })
    }

    fn get_output_type(output: ReturnType) -> syn::Result<TypePath> {
        let ReturnType::Type(_, inner) = output else {
            return Err(syn::Error::new_spanned(
                output,
                "Constructors must return the type they create",
            ));
        };
        match *inner {
            Type::Path(path) => Ok(path),
            other => Err(syn::Error::new_spanned(
                other,
                "Only plain types can be injected",
            )),
        }
    }
    pub fn generate_code(self) -> syn::Result<proc_macro::TokenStream> {
        let create_fn = self.get_create_fn()?;
        let create_meta = self.get_create_meta()?;
        let original = self.body_verbatim;

        Ok(quote! {
            #create_fn
            #create_meta
            #original
        }
        .into())
    }

    fn get_create_fn(&self) -> syn::Result<TokenStream> {
        let constructor_name = &self.constructor_name;
        let mut output_type = self.output_type.clone();
        strip_lifetimes(&mut output_type.path);
        let create_fn_name = self.ns.name_of_create_fn();
        let params = self
            .inputs
            .iter()
            .map(|input| DependentType::from_fn_arg(input).map(|dep| dep.quote_get_call()))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(quote! {
            unsafe fn #create_fn_name(injector: &::injector::Injector) -> ::std::boxed::Box<dyn ::std::any::Any> {
                let constructed = #constructor_name(#(#params),*);
                ::std::boxed::Box::new(unsafe {
                    <#output_type as ::injector::Injectable>::upcast(constructed)
                })
            }
        })
    }

    fn get_create_meta(&self) -> syn::Result<TokenStream> {
        let mut static_type = self.output_type.clone();
        strip_lifetimes(&mut static_type.path);
        let deps = self.inputs.iter().map(DependentType::from_fn_arg);

        utils::quote_inject_meta(static_type, &self.ns, deps)
    }
}
