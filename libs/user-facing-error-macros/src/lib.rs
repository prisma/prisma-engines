extern crate proc_macro;

use darling::{FromDeriveInput, FromVariant};
use once_cell::sync::Lazy;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use regex::Regex;
use std::collections::BTreeSet;
use syn::DeriveInput;

#[proc_macro_derive(UserFacingError, attributes(user_facing))]
pub fn derive_user_facing_error(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    match &input.data {
        syn::Data::Struct(_data) => user_error_derive_on_struct(&input),
        syn::Data::Enum(data) => user_error_derive_on_enum(&input, data),
        _ => syn::Error::new_spanned(input, "derive works only on structs and enums")
            .to_compile_error()
            .into(),
    }
}

fn user_error_derive_on_struct(input: &DeriveInput) -> TokenStream {
    let input = match UserErrorDeriveInput::from_derive_input(&input) {
        Ok(input) => input,
        Err(err) => return err.write_errors().into(),
    };

    let ident = &input.ident;
    let error_code = input.code.as_str();
    let message_template = input.message;
    let template_variables = message_template_variables(message_template.value().as_str(), &message_template.span());

    // Transform from the spec string templates with `${var}` to a rust format string we can use
    // with `format!()`.
    let message_template = message_template.value().replace("${", "{");

    let template_variables = template_variables.iter();

    let output = quote! {
        impl crate::UserFacingError for #ident {
            const ERROR_CODE: &'static str = #error_code;

            fn message(&self) -> String {
                format!(
                    #message_template,
                    #(
                        #template_variables = self.#template_variables
                    ),*
                )
            }
        }
    };

    output.into()
}

fn user_error_derive_on_enum(input: &DeriveInput, data: &syn::DataEnum) -> TokenStream {
    let attributes = match UserErrorEnumDeriveInput::from_derive_input(&input) {
        Ok(attributes) => attributes,
        Err(err) => return err.write_errors().into(),
    };

    let ident = &attributes.ident;
    let error_code = &attributes.code;

    let message_variants = data
        .variants
        .iter()
        .map(|variant| enum_variant_match_branch(ident, variant));

    let output = quote! {
        impl crate::UserFacingError for #ident {
            const ERROR_CODE: &'static str = #error_code;

            fn message(&self) -> String {
                match self {
                    #(#message_variants)*
                }
            }
        }
    };

    output.into()
}

fn enum_variant_match_branch(enum_ident: &syn::Ident, variant: &syn::Variant) -> impl quote::ToTokens {
    let parsed_variant = match UserErrorEnumVariantAttributes::from_variant(variant) {
        Ok(parsed_variant) => parsed_variant,
        Err(err) => return err.write_errors().into(),
    };

    let variant_ident = &parsed_variant.ident;
    let message_template = parsed_variant.message.value().replace("${", "{");

    let variant_field_names: Vec<&syn::Ident> = match &variant.fields {
        syn::Fields::Named(fields) => fields
            .named
            .iter()
            .map(|f| f.ident.as_ref().expect("expect identifier"))
            .collect(),
        tokens => {
            return syn::Error::new_spanned(tokens, "Enum variant fields of user facing errors must be named.")
                .to_compile_error()
                .into()
        }
    };

    quote! {
        #enum_ident::#variant_ident { #(#variant_field_names),* } => format!(
            #message_template,
            #(#variant_field_names = #variant_field_names),*
        ),
    }
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(user_facing))]
struct UserErrorEnumDeriveInput {
    /// The name of the enum.
    ident: syn::Ident,
    /// The error code.
    code: String,
}

#[derive(Debug, FromVariant)]
#[darling(attributes(user_facing))]
struct UserErrorEnumVariantAttributes {
    /// The name of the enum.
    ident: syn::Ident,
    /// The error message format string.
    message: syn::LitStr,
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(user_facing))]
struct UserErrorDeriveInput {
    /// The name of the struct.
    ident: syn::Ident,
    /// The error code.
    code: String,
    /// The error message format string.
    message: syn::LitStr,
}

/// See MESSAGE_VARIABLE_REGEX
const MESSAGE_VARIABLE_REGEX_PATTERN: &str = r##"(?x)
    \$\{  # A curly brace preceded by a dollar sign
    (
        [a-zA-Z0-9_]+  # any number of alphanumeric characters and underscores
    )
    }  # a closing curly brace
"##;

/// The regex for variables in message templates.
static MESSAGE_VARIABLE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(MESSAGE_VARIABLE_REGEX_PATTERN).unwrap());

fn message_template_variables(template: &str, span: &Span) -> BTreeSet<Ident> {
    let captures = MESSAGE_VARIABLE_REGEX.captures_iter(&template);

    captures
        // The unwrap is safe because we know this regex has one capture group.
        .map(|capture| capture.get(1).unwrap())
        .map(|m| Ident::new(m.as_str(), span.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_span() -> proc_macro2::Span {
        proc_macro2::Span::call_site()
    }

    fn assert_template_variables(template: &str, expected: &[&str]) {
        let result: Vec<String> = message_template_variables(template, &new_span())
            .iter()
            .map(|ident| ident.to_string())
            .collect();

        assert_eq!(result, expected);
    }

    #[test]
    fn message_template_variables_works() {
        assert_template_variables("no variables", &[]);
        assert_template_variables("${abc}_def", &["abc"]);
        assert_template_variables("abc${_def}", &["_def"]);
        assert_template_variables("some ${ code } sample", &[] as &[&str]);
        assert_template_variables("positional parameter ${} ", &[] as &[&str]);
        assert_template_variables(
            "Message with ${multiple_variables} to ${substitute}",
            &["multiple_variables", "substitute"],
        );
    }
}
