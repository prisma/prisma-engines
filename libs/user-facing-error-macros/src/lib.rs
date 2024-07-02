extern crate proc_macro;

#[proc_macro_derive(SimpleUserFacingError, attributes(user_facing))]
pub fn derive_simple_user_facing_error(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    let data = match &input.data {
        syn::Data::Struct(data) => data,
        _ => {
            return syn::Error::new_spanned(input, "derive works only on structs")
                .to_compile_error()
                .into()
        }
    };

    if !data.fields.is_empty() {
        return syn::Error::new_spanned(&data.fields, "SimpleUserFacingError implementors cannot have fields")
            .to_compile_error()
            .into();
    }

    let UserErrorDeriveInput { ident, code, message } = match UserErrorDeriveInput::new(&input) {
        Ok(input) => input,
        Err(err) => return err.into_compile_error().into(),
    };

    proc_macro::TokenStream::from(quote::quote! {
        impl serde::Serialize for #ident {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where S: serde::Serializer
            {
                serializer.serialize_none()
            }
        }

        impl crate::SimpleUserFacingError for #ident {
            const ERROR_CODE: &'static str = #code;
            const MESSAGE: &'static str = #message;
        }
    })
}

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

    let template_variables: Box<dyn Iterator<Item = _>> = match &data.fields {
        syn::Fields::Named(named) => Box::new(named.named.iter().map(|field| field.ident.as_ref().unwrap())),
        syn::Fields::Unit => Box::new(std::iter::empty()),
        syn::Fields::Unnamed(unnamed) => {
            return syn::Error::new_spanned(unnamed, "The error fields must be named")
                .to_compile_error()
                .into()
        }
    };

    proc_macro::TokenStream::from(quote::quote! {
        impl crate::UserFacingError for #ident {
            const ERROR_CODE: &'static str = #code;

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

struct UserErrorDeriveInput<'a> {
    /// The name of the struct.
    ident: &'a syn::Ident,
    /// The error code.
    code: syn::LitStr,
    /// The error message format string.
    message: syn::LitStr,
}

impl<'a> UserErrorDeriveInput<'a> {
    fn new(input: &'a syn::DeriveInput) -> Result<Self, syn::Error> {
        let mut code = None;
        let mut message = None;

        for attr in &input.attrs {
            if !attr
                .path()
                .get_ident()
                .map(|ident| ident == "user_facing")
                .unwrap_or(false)
            {
                continue;
            }

            for namevalue in attr.parse_args_with(|stream: &'_ syn::parse::ParseBuffer| {
                syn::punctuated::Punctuated::<syn::MetaNameValue, syn::Token![,]>::parse_terminated(stream)
            })? {
                let litstr = match namevalue.value {
                    syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(litstr), .. }) => litstr,
                    other => {
                        return Err(syn::Error::new_spanned(
                            other,
                            "Expected attribute of the form `#[user_facing(code = \"...\", message = \"...\")]`",
                        ))
                    }
                };

                match namevalue.path.get_ident() {
                    Some(ident) if ident == "code" => {
                        code = Some(litstr);
                    }
                    Some(ident) if ident == "message" => {
                        message = Some(litstr);
                    }
                    other => {
                        return Err(syn::Error::new_spanned(
                            other,
                            "Expected attribute of the form `#[user_facing(code = \"...\", message = \"...\")]`",
                        ))
                    }
                }
            }
        }

        match (message, code) {
            (Some(message), Some(code)) => Ok(UserErrorDeriveInput {
                ident: &input.ident,
                message,
                code,
            }),
            _ => Err(syn::Error::new_spanned(
                input,
                "Expected attribute of the form `#[user_facing(code = \"...\", message = \"...\")]`",
            )),
        }
    }
}
