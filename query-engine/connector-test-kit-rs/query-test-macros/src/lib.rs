extern crate proc_macro;

mod args;
mod attr_map;
mod connector_test;
mod relation_link_test;
mod test_suite;

use args::*;
use connector_test::*;
use proc_macro::TokenStream;
use relation_link_test::*;
use test_suite::*;

#[proc_macro_attribute]
pub fn test_suite(attr: TokenStream, input: TokenStream) -> TokenStream {
    test_suite_impl(attr, input)
}

#[proc_macro_attribute]
pub fn connector_test(attr: TokenStream, input: TokenStream) -> TokenStream {
    connector_test_impl(attr, input)
}

#[proc_macro_attribute]
pub fn relation_link_test(attr: TokenStream, input: TokenStream) -> TokenStream {
    relation_link_test_impl(attr, input)
}
