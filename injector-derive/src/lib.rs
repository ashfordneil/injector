mod attribute_constructor;
mod derive_injectable;

mod utils;

#[proc_macro_derive(Injectable, attributes(has_constructor))]
pub fn derive_injectable(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = match derive_injectable::InjectableDeriveInputs::from_input(input) {
        Ok(input) => input,
        Err(err) => return err.to_compile_error().into(),
    };

    input.derive().unwrap_or_else(|err| err.to_compile_error().into())
}

#[proc_macro_attribute]
pub fn constructor(attr: proc_macro::TokenStream, body: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = match attribute_constructor::ConstructorAttributeInputs::from_input(attr, body) {
        Ok(input) => input,
        Err(err) => return err.to_compile_error().into(),
    };

    input.generate_code().unwrap_or_else(|err| err.to_compile_error().into())
}