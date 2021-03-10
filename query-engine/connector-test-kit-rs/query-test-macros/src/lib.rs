extern crate proc_macro;

use darling::FromMeta;
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn connector_test(attr: TokenStream, input: TokenStream) -> TokenStream {
    connector_test_impl(attr, input)
}

fn connector_test_impl(attr: TokenStream, input: TokenStream) -> TokenStream {
    input
}

#[derive(Debug, FromMeta)]
struct ConnectorTestArgs {
    // schema:
}
