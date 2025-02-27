mod derive_injectable;

#[proc_macro_derive(Injectable, attributes(constructor))]
pub fn derive_injectable(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = match derive_injectable::InjectableDeriveInputs::from_input(input) {
        Ok(input) => input,
        Err(err) => return err.to_compile_error().into(),
    };

    match input.derive() {
        Ok(output) => output.into(),
        Err(err) => err.to_compile_error().into(),
    }
}