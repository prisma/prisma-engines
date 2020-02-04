use super::CONNECTOR_NAMES;
use darling::FromMeta;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, Ident, ItemFn};

#[derive(Debug, FromMeta)]
struct TestEachConnectorArgs {
    /// Comma-separated list of connectors to exclude.
    #[darling(default)]
    ignore: Option<String>,

    #[darling(default)]
    starts_with: Option<String>,

    #[darling(default)]
    log: Option<String>,
}

impl TestEachConnectorArgs {
    fn connectors_to_test(&self) -> impl Iterator<Item = &&str> {
        let ignore = self.ignore.as_ref().map(String::as_str);
        let starts_with = self.starts_with.as_ref().map(String::as_str);

        CONNECTOR_NAMES
            .iter()
            .filter(move |connector_name| match ignore {
                Some(ignore) => !connector_name.starts_with(&ignore),
                None => true,
            })
            .filter(move |connector_name| match starts_with {
                Some(pat) => connector_name.starts_with(pat),
                None => true,
            })
    }
}

pub fn test_each_connector_impl(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attributes_meta: syn::AttributeArgs = parse_macro_input!(attr as AttributeArgs);
    let args = TestEachConnectorArgs::from_list(&attributes_meta);

    let mut test_function = parse_macro_input!(input as ItemFn);
    super::strip_test_attribute(&mut test_function);

    let tests = match args {
        Ok(args) => test_each_connector_async_wrapper_functions(&args, &test_function),
        Err(err) => return err.write_errors().into(),
    };

    let output = quote! {
        #(#tests)*

        #test_function
    };

    output.into()
}

fn test_each_connector_async_wrapper_functions(
    args: &TestEachConnectorArgs,
    test_function: &ItemFn,
) -> Vec<proc_macro2::TokenStream> {
    let test_fn_name = &test_function.sig.ident;
    let test_fn_name_str = format!("{}", test_fn_name);

    let mut tests = Vec::with_capacity(CONNECTOR_NAMES.len());

    let optional_logging_import = args.log.as_ref().map(|_| {
        quote!(
            use tracing_futures::WithSubscriber;
        )
    });
    let optional_logging = args.log.as_ref().map(|log_config| {
        quote! { .with_subscriber(test_setup::logging::test_tracing_subscriber(#log_config)) }
    });

    let optional_unwrap = if super::function_returns_result(&test_function) {
        Some(quote!(.unwrap()))
    } else {
        None
    };

    for connector in args.connectors_to_test() {
        let connector_test_fn_name = Ident::new(&format!("{}_on_{}", test_fn_name, connector), Span::call_site());
        let connector_api_factory = Ident::new(&format!("{}_test_api", connector), Span::call_site());

        let test = quote! {
            #[test]
            fn #connector_test_fn_name() {
                #optional_logging_import

                let fut = async {
                    let api = #connector_api_factory(#test_fn_name_str).await;
                    #test_fn_name(&api).await#optional_unwrap
                }#optional_logging;

                test_setup::runtime::run_with_tokio(fut)
            }
        };

        tests.push(test);
    }

    tests
}
