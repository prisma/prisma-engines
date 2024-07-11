use super::*;
use darling::{ast::NestedMeta, FromMeta};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{parse_macro_input, ItemFn};

pub fn connector_test_impl(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attributes_meta: Vec<NestedMeta> = match NestedMeta::parse_meta_list(attr.into()) {
        Ok(v) => v,
        Err(e) => { return TokenStream::from(darling::Error::from(e).write_errors()); }
    };
    let args = ConnectorTestArgs::from_list(&attributes_meta);
    let args = match args {
        Ok(args) => args,
        Err(err) => return err.write_errors().into(),
    };

    if let Err(err) = args.validate(false) {
        return err.write_errors().into();
    };

    let excluded_features = args.exclude_features.features();
    let db_schemas = args.db_schemas.schemas();
    let db_extensions = args.db_extensions.extensions();
    let only = &args.only;
    let exclude = &args.exclude;
    let handler = args.schema.unwrap().handler_path;

    let mut test_function = parse_macro_input!(input as ItemFn);

    if test_function.sig.inputs.len() != 1 {
        return syn::Error::new_spanned(
            test_function.sig,
            "connector test functions must take exactly one argument: `runner: Runner`.",
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

    // Rename original test function to run_<orig_name>.
    let runner_fn_ident = Ident::new(&format!("run_{test_fn_ident}"), Span::call_site());
    test_function.sig.ident = runner_fn_ident.clone();

    // The test database name is the name used as the database for data source rendering.
    // Combination of test name and test mod name.
    let test_name = test_fn_ident.to_string();
    let suite_name = args.suite.expect("A test must have a test suite.");
    let test_database_name = format!("{suite_name}_{test_name}");
    let capabilities = args.capabilities.idents;

    let referential_override = match args.relation_mode.or(args.referential_integrity) {
        Some(ref_override) => {
            let wat = ref_override.to_string();
            quote! { Some(#wat.to_string()) }
        }
        None => quote! { None },
    };

    // The actual test is a shell function that gets the name of the original function,
    // which is then calling `{orig_name}_run` in the end (see `runner_fn_ident`).
    let test = quote! {
        #[test]
        fn #test_fn_ident() {
            query_tests_setup::run_connector_test(
                #test_database_name,
                &[#only],
                &[#exclude],
                enumflags2::make_bitflags!(ConnectorCapability::{#(#capabilities)|*}),
                &[#(#excluded_features),*],
                #handler,
                &[#(#db_schemas),*],
                &[#(#db_extensions),*],
                #referential_override,
                #runner_fn_ident,
            );
        }

        #test_function
    };

    test.into()
}
