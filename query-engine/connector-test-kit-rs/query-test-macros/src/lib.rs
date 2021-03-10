extern crate proc_macro;

use darling::FromMeta;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn connector_test(attr: TokenStream, input: TokenStream) -> TokenStream {
    connector_test_impl(attr, input)
}

fn connector_test_impl(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let test_function = parse_macro_input!(input as ItemFn);
    let ident = test_function.sig.ident.clone();

    let runner_ident = Ident::new(&format!("run_{}", ident.to_string()), Span::call_site());
    let test = quote! {
        #[test]
        fn #runner_ident() {
            let runner = Runner::load();

            #ident(&runner)
        }

        #test_function
    };

    test.into()
}

#[derive(Debug)]
struct SchemaHandler {
    handler: fn() -> String,
}

#[derive(Debug, FromMeta)]
struct ConnectorTestArgs {
    #[darling(default)]
    schema: Option<SchemaHandler>,
}

impl darling::FromMeta for SchemaHandler {
    fn from_word() -> Result<Self, darling::Error> {
        todo!("woot")
    }
}
