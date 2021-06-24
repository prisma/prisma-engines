extern crate proc_macro;

mod args;
mod attr_map;
mod connector_test;
mod relation_link_test;
mod test_suite;
mod utils;

use args::*;
use connector_test::*;
use proc_macro::TokenStream;
use proc_macro2::Span;
use query_tests_setup::TestError;
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

trait IntoDarlingError<T> {
    fn into_darling_error(self, span: &Span) -> std::result::Result<T, darling::Error>;
}

impl<T> IntoDarlingError<T> for std::result::Result<T, TestError> {
    fn into_darling_error(self, span: &Span) -> std::result::Result<T, darling::Error> {
        self.map_err(|err| match err {
            TestError::ParseError(msg) => darling::Error::custom(&format!("Parsing error: {}.", msg)).with_span(span),
            TestError::ConfigError(msg) => {
                darling::Error::custom(&format!("Configuration error: {}.", msg)).with_span(span)
            }
            err => unimplemented!("{:?} not yet handled for test setup compilation", err),
        })
    }
}
