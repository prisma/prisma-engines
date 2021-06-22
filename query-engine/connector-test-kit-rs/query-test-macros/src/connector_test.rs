use crate::{args::ConnectorTestGenArgs, ConnectorTestArgs};
use darling::FromMeta;
use itertools::Itertools;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use query_tests_setup::{schema_with_relation::*, ConnectorTag, ConnectorTagInterface};
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
    let capabilities: Vec<_> = args
        .capabilities
        .idents
        .into_iter()
        .map(|cap| {
            quote! {
                ConnectorCapability::#cap
            }
        })
        .collect();

    // The actual test is a shell function that gets the name of the original function,
    // which is then calling `{orig_name}_run` in the end (see `runner_fn_ident`).
    let test = quote! {
        #[test]
        fn #test_fn_ident() {
            let config = &query_tests_setup::CONFIG;
            let enabled_connectors = vec![
                #connectors
            ];

            let capabilities: Vec<ConnectorCapability> = vec![
                #(#capabilities),*
            ];

            if ConnectorTag::should_run(&config, &enabled_connectors, &capabilities, #test_name) {
                let template = #handler();
                let datamodel = query_tests_setup::render_test_datamodel(config, #test_database, template);
                let connector = config.test_connector_tag().unwrap();

                query_tests_setup::run_with_tokio(async move {
                    tracing::debug!("Used datamodel:\n {}", datamodel.clone().yellow());
                    let runner = Runner::load(config.runner(), datamodel.clone(), connector).await.unwrap();
                    query_tests_setup::setup_project(&datamodel).await.unwrap();
                    #runner_fn_ident(&runner).await.unwrap();
                }.with_subscriber(test_tracing_subscriber(std::env::var("LOG_LEVEL").unwrap_or("info".to_string()))));
            }
        }

        #test_function
    };

    test.into()
}

pub fn connector_schema_gen_impl(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attributes_meta: syn::AttributeArgs = parse_macro_input!(attr as AttributeArgs);
    let args = ConnectorTestGenArgs::from_list(&attributes_meta);
    let args = match args {
        Ok(args) => args,
        Err(err) => return err.write_errors().into(),
    };

    if let Err(err) = args.validate(false) {
        return err.write_errors().into();
    };

    dbg!(&args);

    let connectors = args.connectors_to_test();
    // let handler = args.schema.unwrap().handler_path;

    // Renders the connectors as list to use in the code.
    let connectors = connectors.into_iter().map(quote_connector).fold1(|aggr, next| {
        quote! {
            #aggr, #next
        }
    });
    let schem_gen = args.gen.unwrap();
    let datamodels: Vec<_> = schema_with_relation(schem_gen.on_parent, schem_gen.on_child, schem_gen.without_parent);

    let mut test_function = parse_macro_input!(input as ItemFn);

    if test_function.sig.inputs.len() != 2 {
        return syn::Error::new_spanned(
            test_function.sig,
            "connector test functions must take exactly two arguments: `runner: &Runner, dm: &DatamodelWithParams`.",
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
    let capabilities: Vec<_> = args
        .capabilities
        .idents
        .into_iter()
        .map(|cap| {
            quote! {
                ConnectorCapability::#cap
            }
        })
        .collect();

    let test_shells: Vec<proc_macro2::TokenStream> = datamodels.into_iter().enumerate().map(|(i, dm)| {
        // The shell function retains the name of the original test definition.
        let test_fn_ident = Ident::new(&format!("{}_{}", test_fn_ident.to_string(), i), Span::call_site());
        let datamodel: proc_macro2::TokenStream = format!(r#""{}""#, dm.datamodel).parse().unwrap();
        let dm_with_params: String = dm.into();
        let test_database = format!("{}_{}_{}", suite_name, test_name, i);

        let ts = quote! {
            #[test]
            fn #test_fn_ident() {
                let config = &query_tests_setup::CONFIG;
                let enabled_connectors = vec![
                    #connectors
                ];

                let capabilities: Vec<ConnectorCapability> = vec![
                    #(#capabilities),*
                ];
                let template = #datamodel.to_string();
                let dm_with_params_json = #dm_with_params.to_string();

                if ConnectorTag::should_run(&config, &enabled_connectors, &capabilities, #test_name) {
                    let datamodel = query_tests_setup::render_test_datamodel(config, #test_database, template);
                    let connector = config.test_connector_tag().unwrap();

                    query_tests_setup::run_with_tokio(async move {
                        tracing::debug!("Used datamodel:\n {}", datamodel.clone().yellow());
                        let runner = Runner::load(config.runner(), datamodel.clone(), connector).await.unwrap();
                        let dm_with_params = DatamodelWithParams::from(dm_with_params_json);
                        query_tests_setup::setup_project(&datamodel).await.unwrap();
                        #runner_fn_ident(&runner, &dm_with_params).await.unwrap();
                    }.with_subscriber(test_tracing_subscriber(std::env::var("LOG_LEVEL").unwrap_or("info".to_string()))));
                }
            }
        };

        ts.into()
    }).collect();

    // Expand all test shell functions
    let all_funcs: proc_macro2::TokenStream = test_shells
        .into_iter()
        .fold1(|aggr, next| {
            quote! {
                #aggr

                #next
            }
        })
        .unwrap()
        .into();

    // Expand the actual test runner
    let all_funcs_with_original_func = quote! {
        #all_funcs

        #test_function
    };

    all_funcs_with_original_func.into()
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
