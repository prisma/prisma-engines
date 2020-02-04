use darling::FromMeta;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::str::FromStr;
use syn::{parse_macro_input, spanned::Spanned, AttributeArgs, Ident, ItemFn};
use test_setup::connectors::{Capabilities, Connector, CONNECTORS};

#[derive(Debug, FromMeta)]
struct TestEachConnectorArgs {
    #[darling(default)]
    ignore: Option<String>,

    #[darling(default)]
    starts_with: Option<String>,

    #[darling(default)]
    log: Option<String>,

    #[darling(default)]
    capabilities: CapabilitiesWrapper,
}

#[derive(Debug)]
struct CapabilitiesWrapper(Capabilities);

impl Default for CapabilitiesWrapper {
    fn default() -> Self {
        CapabilitiesWrapper(Capabilities::empty())
    }
}

impl darling::FromMeta for CapabilitiesWrapper {
    fn from_list(items: &[syn::NestedMeta]) -> Result<Self, darling::Error> {
        let mut capabilities = Capabilities::empty();

        for item in items {
            match item {
                syn::NestedMeta::Lit(syn::Lit::Str(s)) => {
                    let s = s.value();
                    let capability = Capabilities::from_str(&s)
                        .map_err(|err| darling::Error::unknown_value(&err.to_string()).with_span(&item.span()))?;
                    capabilities.insert(capability);
                }
                syn::NestedMeta::Lit(other) => {
                    return Err(darling::Error::unexpected_lit_type(other).with_span(&other.span()))
                }
                syn::NestedMeta::Meta(meta) => {
                    return Err(darling::Error::unsupported_shape("Expected string literal").with_span(&meta.span()))
                }
            }
        }

        Ok(CapabilitiesWrapper(capabilities))
    }
}

impl TestEachConnectorArgs {
    fn connectors_to_test(&self) -> impl Iterator<Item = &Connector> {
        let ignore = self.ignore.as_ref().map(String::as_str);
        let starts_with = self.starts_with.as_ref().map(String::as_str);

        CONNECTORS
            .all()
            .filter(move |connector| match ignore {
                Some(ignore) => !connector.name().starts_with(&ignore),
                None => true,
            })
            .filter(move |connector| match starts_with {
                Some(pat) => connector.name().starts_with(pat),
                None => true,
            })
            .filter(move |connector| connector.capabilities.contains(self.capabilities.0))
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
    let test_fn_name_str = test_fn_name.to_string();

    let mut tests = Vec::with_capacity(CONNECTORS.len());

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
        let connector_test_fn_name =
            Ident::new(&format!("{}_on_{}", test_fn_name, connector.name()), Span::call_site());
        let connector_api_factory = Ident::new(connector.test_api(), Span::call_site());

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

    if tests.is_empty() {
        return vec![
            syn::Error::new_spanned(test_function, "All connectors were filtered out for this test.")
                .to_compile_error()
                .into(),
        ];
    }

    tests
}
