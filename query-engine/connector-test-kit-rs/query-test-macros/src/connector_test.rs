use crate::ConnectorTestArgs;
use darling::FromMeta;
use itertools::Itertools;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use query_tests_setup::{ConnectorTag, ConnectorTagInterface};
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, ItemFn};

pub fn connector_test_impl(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attributes_meta: syn::AttributeArgs = parse_macro_input!(attr as AttributeArgs);
    let args = ConnectorTestArgs::from_list(&attributes_meta);
    let args = match args {
        Ok(args) => args,
        Err(err) => return err.write_errors().into(),
    };

    if let Err(err) = args.validate(false) {
        return err.write_errors().into();
    };

    let connectors = args.connectors_to_test();
    let handler = args.schema.unwrap().handler_path;

    // Renders the connectors as list to use in the code.
    let connectors = connectors.into_iter().map(quote_connector).fold1(|aggr, next| {
        quote! {
            #aggr, #next
        }
    });

    let mut test_function = parse_macro_input!(input as ItemFn);

    if test_function.sig.inputs.len() != 1 {
        return syn::Error::new_spanned(
            test_function.sig,
            "connector test functions must take exactly one argument: `runner: &Runner`.",
        )
        .to_compile_error()
        .into();
    }

    if test_function.sig.asyncness.is_none() {
        return syn::Error::new_spanned(test_function.sig, "connector test functions must be async")
            .to_compile_error()
            .into();
    }

    // The shell function retains the name of the original test definition.
    let test_fn_ident = test_function.sig.ident.clone();

    // Rename original test function to run_<orig_name>.
    let runner_fn_ident = Ident::new(&format!("run_{}", test_fn_ident.to_string()), Span::call_site());
    test_function.sig.ident = runner_fn_ident.clone();

    // The test database name is the name used as the database for data source rendering.
    // Combination of test name and test mod name.
    let test_name = test_fn_ident.to_string();
    let suite_name = args.suite.expect("A test must have a test suite.");
    let test_database = format!("{}_{}", suite_name, test_name);

    // The actual test is a shell function that gets the name of the original function,
    // which is then calling `{orig_name}_run` in the end (see `runner_fn_ident`).
    let test = quote! {
        #[test]
        fn #test_fn_ident() {
            let config = &query_tests_setup::CONFIG;
            let enabled_connectors = vec![
                #connectors
            ];

            if !ConnectorTag::should_run(&config, &enabled_connectors) {
                tracing::info!("Skipping test '{}', current test connector is not enabled.", #test_name);
                return
            }

            let template = #handler();
            let datamodel = query_tests_setup::render_test_datamodel(config, #test_database, template);

            query_tests_setup::run_with_tokio(async move {
                tracing::debug!("Used datamodel:\n {}", datamodel.clone().yellow());
                let runner = Runner::load(config.runner(), datamodel.clone()).await.unwrap();
                query_tests_setup::setup_project(&datamodel).await.unwrap();
                #runner_fn_ident(&runner).await.unwrap();
            }.with_subscriber(test_tracing_subscriber(std::env::var("LOG_LEVEL").unwrap_or("info".to_string()))));
        }

        #test_function
    };

    test.into()
}

fn quote_connector(tag: ConnectorTag) -> proc_macro2::TokenStream {
    let (connector, version) = tag.as_parse_pair();

    match version {
        Some(version) => quote! {
            ConnectorTag::try_from((#connector, Some(#version))).unwrap()
        },
        None => quote! {
            ConnectorTag::try_from(#connector).unwrap()
        },
    }
}
