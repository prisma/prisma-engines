use darling::FromMeta;
use once_cell::sync::Lazy;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::str::FromStr;
use syn::{parse_macro_input, spanned::Spanned, AttributeArgs, Ident, ItemFn};
use test_setup::connectors::{Capabilities, Connector, Tags, CONNECTORS};

static TAGS_FILTER: Lazy<Tags> = Lazy::new(|| {
    let tags_str = std::env::var("TEST_EACH_CONNECTOR_TAGS").ok();
    let mut tags = Tags::empty();

    if let Some(tags_str) = tags_str {
        for tag_str in tags_str.split(',') {
            let tag = Tags::from_str(tag_str).unwrap();
            tags |= tag;
        }
    }

    tags
});

#[derive(Debug, FromMeta)]
struct TestEachConnectorArgs {
    /// If present, setup tracing logging with the passed in configuration string.
    #[darling(default)]
    log: Option<String>,

    /// If present, run only the tests for the connectors with all of the passed
    /// in capabilities.
    #[darling(default)]
    capabilities: CapabilitiesWrapper,

    /// If present, run only the tests for the connectors with any of the passed
    /// in tags.
    #[darling(default)]
    tags: TagsWrapper,

    /// Optional list of tags to ignore.
    #[darling(default)]
    ignore: TagsWrapper,
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

#[derive(Debug)]
struct TagsWrapper(Tags);

impl Default for TagsWrapper {
    fn default() -> Self {
        TagsWrapper(Tags::empty())
    }
}

impl darling::FromMeta for TagsWrapper {
    fn from_list(items: &[syn::NestedMeta]) -> Result<Self, darling::Error> {
        let mut tags = Tags::empty();

        for item in items {
            match item {
                syn::NestedMeta::Lit(syn::Lit::Str(s)) => {
                    let s = s.value();
                    let tag = Tags::from_str(&s)
                        .map_err(|err| darling::Error::unknown_value(&err.to_string()).with_span(&item.span()))?;
                    tags.insert(tag);
                }
                syn::NestedMeta::Lit(other) => {
                    return Err(darling::Error::unexpected_lit_type(other).with_span(&other.span()))
                }
                syn::NestedMeta::Meta(meta) => {
                    return Err(darling::Error::unsupported_shape("Expected string literal").with_span(&meta.span()))
                }
            }
        }

        Ok(TagsWrapper(tags))
    }
}

impl TestEachConnectorArgs {
    fn connectors_to_test(&self) -> impl Iterator<Item = &Connector> {
        CONNECTORS
            .all()
            .filter(move |connector| connector.capabilities.contains(self.capabilities.0))
            .filter(move |connector| TAGS_FILTER.is_empty() || connector.tags.contains(*TAGS_FILTER))
            .filter(move |connector| self.tags.0.is_empty() || connector.tags.intersects(self.tags.0))
            .filter(move |connector| !connector.tags.intersects(self.ignore.0))
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

    if tests.is_empty() && TAGS_FILTER.is_empty() {
        return vec![
            syn::Error::new_spanned(test_function, "All connectors were filtered out for this test.")
                .to_compile_error(),
        ];
    }

    tests
}
