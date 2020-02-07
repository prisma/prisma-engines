extern crate proc_macro;

mod test_each_connector;

use darling::FromMeta;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, Ident, ItemFn};

#[derive(Debug, FromMeta)]
struct TestOneConnectorArgs {
    /// The name of the connector to test.
    connector: String,

    #[darling(default)]
    log: Option<String>,
}

#[proc_macro_attribute]
pub fn test_each_connector(attr: TokenStream, input: TokenStream) -> TokenStream {
    test_each_connector::test_each_connector_impl(attr, input)
}

#[proc_macro_attribute]
pub fn test_one_connector(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attributes_meta: syn::AttributeArgs = parse_macro_input!(attr as AttributeArgs);
    let args = TestOneConnectorArgs::from_list(&attributes_meta).unwrap();

    let mut test_function = parse_macro_input!(input as ItemFn);
    strip_test_attribute(&mut test_function);

    let async_test: bool = test_function.sig.asyncness.is_some();
    let test_impl_name = &test_function.sig.ident;
    let test_impl_name_str = format!("{}", test_impl_name);
    let test_fn_name = Ident::new(
        &format!("{}_on_{}", &test_function.sig.ident, args.connector),
        Span::call_site(),
    );
    let api_factory = Ident::new(&format!("{}_test_api", args.connector), Span::call_site());
    let optional_unwrap = if function_returns_result(&test_function) {
        Some(quote!(.unwrap()))
    } else {
        None
    };

    let output = if async_test {
        let optional_logging_import = args.log.as_ref().map(|_| {
            quote!(
                use tracing_futures::WithSubscriber;
            )
        });
        let optional_logging = args.log.as_ref().map(|log_config| {
            quote! { .with_subscriber(test_setup::logging::test_tracing_subscriber(#log_config)) }
        });

        quote! {
            #[test]
            fn #test_fn_name() {
                #optional_logging_import

                let fut = async {
                    let api = #api_factory(#test_impl_name_str).await;
                    #test_impl_name(&api)#optional_logging.await#optional_unwrap
                };

                test_setup::runtime::run_with_tokio(fut)
            }

            #test_function
        }
    } else {
        quote! {
            #[test]
            fn #test_fn_name() {
                let api = #api_factory(#test_impl_name_str);

                #test_impl_name(&api)
            }

            #test_function
        }
    };

    output.into()
}

fn function_returns_result(func: &ItemFn) -> bool {
    match func.sig.output {
        syn::ReturnType::Default => false,
        // just assume it's a result
        syn::ReturnType::Type(_, _) => true,
    }
}

/// We do this because Intellij only recognizes functions annotated with #[test] *before* macro expansion as tests. This way we can add it manually, and the test macro will strip it.
fn strip_test_attribute(function: &mut ItemFn) {
    let new_attrs = function
        .attrs
        .drain(..)
        .filter(|attr| attr.path.segments.iter().last().unwrap().ident != "test")
        .collect();

    function.attrs = new_attrs;
}
