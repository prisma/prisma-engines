extern crate proc_macro;

use darling::FromMeta;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, Ident, ItemFn};

#[derive(Debug, FromMeta)]
struct TestOneConnectorArgs {
    /// The name of the connector to test.
    #[darling(default)]
    connector: String,
}

const CONNECTOR_NAMES: &[&'static str] = &["postgres", "mysql", "sqlite"];

#[derive(Debug, FromMeta)]
struct TestEachConnectorArgs {
    /// Comma-separated list of connectors to exclude.
    #[darling(default)]
    ignore: Option<String>,
}

#[proc_macro_attribute]
pub fn test_each_connector(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attributes_meta: syn::AttributeArgs = parse_macro_input!(attr as AttributeArgs);
    let args = TestEachConnectorArgs::from_list(&attributes_meta);

    match args {
        Ok(_) => (),
        Err(err) => panic!("{}", err),
    };

    let test_function = parse_macro_input!(input as ItemFn);
    let test_fn_name = &test_function.sig.ident;

    let mut tests = Vec::new();

    for connector in CONNECTOR_NAMES {
        let connector_test_fn_name = Ident::new(&format!("{}_on_{}", test_fn_name, connector), Span::call_site());
        let connector_api_factory = Ident::new(&format!("{}_test_api", connector), Span::call_site());

        let test = quote! {
            #[test]
            fn #connector_test_fn_name() {
                let api = #connector_api_factory();

                #test_fn_name(&api)
            }
        };

        tests.push(test);
    }

    let output = quote! {
        #(#tests)*

        #test_function
    };

    output.into()
}
