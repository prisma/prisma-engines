use super::*;
use crate::utils::quote_connector;
use darling::FromMeta;
use itertools::Itertools;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use query_tests_setup::schema_with_relation;
use quote::quote;
use std::convert::TryInto;
use syn::{parse_macro_input, AttributeArgs, ItemFn};

/// Generates a test shell function for each datamodels. Each of these test shells function will call the original test function.
/// Below is a representation in pseudo-code of the final generated code:
///
/// Original code:
/// ```ignore
/// #[relation_link_test(on_parent = "ToOneOpt", on_child = "ToOneOpt")]
/// async fn my_fancy_test(runner: Runner, t: &DatamodelWithParams) -> TestResult<()> {
///   assert_eq!(true, true);
/// }
/// ```
/// Generated code:
///  ```ignore
/// #[test]
/// async fn my_fancy_test_1() -> {
///   setup_database().await?;
///   run_my_fancy_test(runner, t).await?;
/// }
///
/// #[test]
/// async fn my_fancy_test_2() -> {
///   setup_database().await?;
///   run_my_fancy_test(runner, t).await?;
/// }
///
/// #[test]
/// async fn my_fancy_test_n() -> {
///   setup_database().await?;
///   run_my_fancy_test(runner, t).await?;
/// }
///
/// async fn run_my_fancy_test(runner: Runner, t: &DatamodelWithParams) -> TestResult<()> {
///   assert_eq!(true, true);
/// }
/// ```
pub fn relation_link_test_impl(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attributes_meta: syn::AttributeArgs = parse_macro_input!(attr as AttributeArgs);
    let args = RelationLinkTestArgs::from_list(&attributes_meta);
    let args = match args {
        Ok(args) => args,
        Err(err) => return err.write_errors().into(),
    };

    if let Err(err) = args.validate(false) {
        return err.write_errors().into();
    };

    // Renders the connectors as list to use in the code.
    let connectors = args.connectors_to_test();
    let connectors = connectors.into_iter().map(quote_connector).fold1(|aggr, next| {
        quote! {
            #aggr, #next
        }
    });

    let mut test_function = parse_macro_input!(input as ItemFn);
    if test_function.sig.inputs.len() != 2 {
        return syn::Error::new_spanned(
            test_function.sig,
            "connector test functions must take exactly two arguments: `runner: Runner, dm: &DatamodelWithParams`.",
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

    // Generates multiple datamodels and their associated required capabilities
    let (datamodels, required_capabilities) = schema_with_relation(
        args.on_parent.relation_field(),
        args.on_child.relation_field(),
        args.id_only,
    );

    if datamodels.is_empty() {
        panic!("No datamodel were generated")
    }

    let test_shells = datamodels.into_iter().enumerate().map(|(i, dm)| {
        // The shell function retains the name of the original test definition.
        let test_fn_ident = Ident::new(&format!("{}_{}", test_fn_ident.to_string(), i), Span::call_site());
        let datamodel: proc_macro2::TokenStream = format!(r#""{}""#, dm.datamodel())
            .parse()
            .expect("Could not parse the datamodel");
        let dm_with_params: String = dm.try_into().expect("Could not serialize json");
        let test_database = format!("{}_{}_{}", suite_name, test_name, i);
        let required_capabilities = required_capabilities
            .get(i)
            .expect("Could not find some required capabilities")
            .iter()
            .map(|cap| format!("{}", cap))
            .collect::<Vec<_>>();

        let ts = quote! {
            #[test]
            fn #test_fn_ident() {
              query_tests_setup::run_relation_link_test(
                  vec![#connectors],
                  &mut vec![#(#capabilities),*],
                  vec![#(#required_capabilities),*],
                  #datamodel,
                  #dm_with_params,
                  #test_name,
                  #test_database,
                  #runner_fn_ident
                )
            }
        };

        ts
    });

    let all_funcs: proc_macro2::TokenStream = test_shells
        .fold1(|aggr, next| {
            quote! {
                #aggr

                #next
            }
        })
        .unwrap();

    let all_funcs_with_original_func = quote! {
        #all_funcs

        #test_function
    };

    all_funcs_with_original_func.into()
}
