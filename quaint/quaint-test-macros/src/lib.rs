extern crate proc_macro;

mod test_each_connector;

use proc_macro::TokenStream;
use syn::ItemFn;

#[proc_macro_attribute]
pub fn test_each_connector(attr: TokenStream, input: TokenStream) -> TokenStream {
    test_each_connector::test_each_connector_impl(attr, input)
}

fn function_returns_result(func: &ItemFn) -> bool {
    match func.sig.output {
        syn::ReturnType::Default => false,
        // just assume it's a result
        syn::ReturnType::Type(_, _) => true,
    }
}

/// We do this because Intellij only recognizes functions annotated with #[test]
/// *before* macro expansion as tests. This way we can add it manually, and the
/// test macro will strip it.
fn strip_test_attribute(function: &mut ItemFn) {
    let new_attrs = function
        .attrs
        .drain(..)
        .filter(|attr| attr.path.segments.iter().last().unwrap().ident != "test")
        .collect();

    function.attrs = new_attrs;
}
