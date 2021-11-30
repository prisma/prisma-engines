extern crate proc_macro;
use syn::{Attribute, ItemStruct};

/// This parses the list of files, looks for all structs and generates a vector of error types:
///   pub fn error_list() -> Vec<UserErrorType> {
///        Vec::from([
///             UserErrorType {
///                 name: "InputValueTooLong",
///                 code: "P2000"
///             },
///             UserErrorType {
///                 name: "RecordNotFound",
///                 code: "P2001"
///             },
///         ])
///     }
///
pub fn parse_files(srcs: Vec<String>) -> String {
    let errors: Vec<_> = vec![];

    let error_infos = srcs.iter().fold(errors, |mut errors, src| {
        let file = syn::parse_file(src).expect("Unable to parse file");

        file.items.iter().for_each(|i| {
            if let syn::Item::Struct(struct_item) = i {
                let (name, code) = create_name_and_code(struct_item.clone()).unwrap();
                errors.push(quote::quote! {
                    UserErrorType {
                        name: #name,
                        code: #code
                    }
                });
            }
        });

        errors
    });

    let output_file = quote::quote! {

        pub fn error_list() -> Vec<UserErrorType> {
            Vec::from([
                #(
                    #error_infos
                ),*
            ])

        }
    };

    output_file.to_string()
}

fn create_name_and_code(item_struct: ItemStruct) -> Result<(String, String), syn::Error> {
    match UserErrorDeriveInput::parse_input_attr(&item_struct.attrs).unwrap() {
        (Some(_message), Some(code)) => Ok((format!("{}", item_struct.ident), code.value())),
        _ => Err(syn::Error::new_spanned(
            item_struct,
            "Expected attribute of the form `#[user_facing(code = \"...\", message = \"...\")]`",
        )),
    }
}

pub struct UserErrorDeriveInput<'a> {
    /// The name of the struct.
    pub ident: &'a syn::Ident,
    /// The error code.
    pub code: syn::LitStr,
    /// The error message format string.
    pub message: syn::LitStr,
}

impl<'a> UserErrorDeriveInput<'a> {
    pub fn new(input: &'a syn::DeriveInput) -> Result<Self, syn::Error> {
        match Self::parse_input_attr(&input.attrs)? {
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

    fn parse_input_attr(attrs: &[Attribute]) -> Result<(Option<syn::LitStr>, Option<syn::LitStr>), syn::Error> {
        let mut code = None;
        let mut message = None;
        for attr in attrs {
            if !attr
                .path
                .get_ident()
                .map(|ident| ident == "user_facing")
                .unwrap_or(false)
            {
                continue;
            }

            for namevalue in attr.parse_args_with(|stream: &'_ syn::parse::ParseBuffer| {
                syn::punctuated::Punctuated::<syn::MetaNameValue, syn::Token![,]>::parse_terminated(stream)
            })? {
                let litstr = match namevalue.lit {
                    syn::Lit::Str(litstr) => litstr,
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

        Ok((message, code))
    }
}
