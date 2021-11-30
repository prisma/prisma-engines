extern crate proc_macro;
use user_facing_error_parsing::UserErrorDeriveInput;

#[proc_macro_derive(UserFacingError, attributes(user_facing))]
pub fn derive_user_facing_error(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    let data = match &input.data {
        syn::Data::Struct(data) => data,
        _ => {
            return syn::Error::new_spanned(input, "derive works only on structs")
                .to_compile_error()
                .into()
        }
    };

    let UserErrorDeriveInput { ident, code, message } = match UserErrorDeriveInput::new(&input) {
        Ok(input) => input,
        Err(err) => return err.into_compile_error().into(),
    };

    let field_idents = match &data.fields {
        syn::Fields::Named(named) => named.named.iter().map(|field| field.ident.as_ref().unwrap()).collect(),
        syn::Fields::Unit => Vec::new(),
        syn::Fields::Unnamed(unnamed) => {
            return syn::Error::new_spanned(unnamed, "The error fields must be named")
                .to_compile_error()
                .into()
        }
    };

    let fields = field_idents.iter().map(|field| {
        let key = format!("{}", field);
        quote::quote! {
           error_fields.insert(#key.to_string(), serde_json::to_value(&self.#field).unwrap());
        }
    });

    let template_variables = field_idents.iter();
    let error_name = format!("{}", ident);

    proc_macro::TokenStream::from(quote::quote! {

        impl crate::UserFacingError for #ident {
            const ERROR_CODE: &'static str = #code;

            fn error_details(&self) -> ErrorDetails {
                use std::collections::HashMap;
                use serde_json::json;

                let mut error_fields: HashMap<String, serde_json::Value> = HashMap::new();

                #(
                    #fields
                )*

                ErrorDetails {
                    name: #error_name,
                    code: Self::ERROR_CODE,
                    fields: json!(error_fields)
                }
            }

            fn message(&self) -> String {
                format!(
                    #message,
                    #(
                        #template_variables = self.#template_variables
                    ),*
                )
            }
        }
    })
}
