extern crate proc_macro;

mod test_each_connector;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn test_connector(attr: TokenStream, input: TokenStream) -> TokenStream {
    test_each_connector::test_connector_impl(attr, input)
}
