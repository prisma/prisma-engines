use super::*;
use crate::ensure_db_names::UNIQUE_TEST_DATABASE_NAMES;
use darling::{FromMeta, ast::NestedMeta};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{ItemFn, parse_macro_input};

pub fn relation_link_test_impl(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attributes_meta = match NestedMeta::parse_meta_list(attr.into()) {
        Ok(meta) => meta,
        Err(err) => return err.into_compile_error().into(),
    };

    let args = match RelationLinkTestArgs::from_list(&attributes_meta) {
        Ok(args) => args,
        Err(err) => return err.write_errors().into(),
    };

    if let Err(err) = args.validate(false) {
        return err.write_errors().into();
    };

    let only = args.only;
    let exclude = args.exclude;
    let only_executors = args.only_executors.as_ref();
    let excluded_executors = args.exclude_executors.as_ref();
    let id_only = args.id_only;
    let on_parent = args.on_parent.relation_field;
    let on_child = args.on_child.relation_field;

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
    let test_fn_ident = test_function.sig.ident;
    let test_fn_ident_string = test_fn_ident.to_string();

    // Rename original test function to run_<orig_name>.
    let runner_fn_ident = Ident::new(&format!("run_{test_fn_ident}"), Span::call_site());
    test_function.sig.ident = runner_fn_ident.clone();

    // The test database name is the name used as the database for data source rendering.
    // Combination of test name and test mod name.
    let test_name = test_fn_ident.to_string();
    let suite_name = args.suite.expect("A test must have a test suite.");
    let required_capabilities = &args.capabilities.idents;

    let test_database_name = format!("{suite_name}_{test_name}_#");
    UNIQUE_TEST_DATABASE_NAMES.ensure_unique(&test_database_name, &suite_name, &test_name);

    let ts = quote! {
        #[test]
        fn #test_fn_ident() {
            query_tests_setup::run_relation_link_test(
                #on_parent,
                #on_child,
                #id_only,
                &[#only],
                &[#exclude],
                &[#(#only_executors),*],
                &[#(#excluded_executors),*],
                enumflags2::make_bitflags!(ConnectorCapability::{#(#required_capabilities)|*}),
                (#suite_name, #test_name),
                #runner_fn_ident,
                #test_fn_ident_string
            )
        }

        #test_function
    };

    ts.into()
}
