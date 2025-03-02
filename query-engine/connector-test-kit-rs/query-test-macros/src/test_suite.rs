use crate::{attr_map::NestedAttrMap, ConnectorTestArgs};
use darling::{ast::NestedMeta, FromMeta, ToTokens};
use proc_macro::TokenStream;
use quote::quote;
use std::collections::hash_map::Entry;
use syn::{
    parse_macro_input, parse_quote, spanned::Spanned, token::Paren, Item, ItemMod, MacroDelimiter, Meta, MetaList,
};

/// What does this do?
/// Test attributes (like `schema(handler)`, `only`, ...) can be defined on the test (`connector_test`) or on the module.
/// Setting them on the module allows to define defaults that apply to all `connector_test`s in the module.
/// Individual tests can still set their attributes, which will take precedence and overwrite the defaults.
/// This macro merges the attributes of the module and writes them to the test function.
/// Example: If the following test suite definition is given:
/// ```ignore
/// #[test_suite(schema(handler), exclude(SqlServer))]
/// mod test_mod {
///     #[connector_test]
///     async fn test_a() { ... }
///
///     #[connector_test(suite = "other_tests", schema(other_handler), only(Postgres)]
///     async fn test_b() { ... }
/// }
/// ```
/// Will be rewritten to:
/// ```ignore
/// mod test_mod {
///     #[connector_test(suite = "test_mod", schema(handler), exclude(SqlServer))]
///     async fn test_a() { ... }
///
///     #[connector_test(suite = "other_tests", schema(other_handler), only(Postgres)]
///     async fn test_b() { ... }
/// }
/// ```
/// As can be seen with the example, there are some rules regarding `only` and `exclude`, but the gist is that
/// only one connector definition can be present, and since test_b already defines a connector tag rule, this one
/// takes precedence. Same with the `suite` and `schema` attributes - they overwrite the defaults of the mod.
/// A notable expansion is that the name of the test mod is added as `suite = <name>` to the tests.
pub fn test_suite_impl(attr: TokenStream, input: TokenStream) -> TokenStream {
    // Validate input by simply parsing it, which will point out invalid fields and connector names etc.
    let attributes_meta = match NestedMeta::parse_meta_list(attr.into()) {
        Ok(meta) => meta,
        Err(err) => return err.into_compile_error().into(),
    };

    let args = match ConnectorTestArgs::from_list(&attributes_meta) {
        Ok(args) => args,
        Err(err) => return err.write_errors().into(),
    };

    if let Err(err) = args.validate(true) {
        return err.write_errors().into();
    };
    // end validation

    let mut test_module = parse_macro_input!(input as ItemMod);
    let module_name = test_module.ident.to_string();
    let mut module_attrs = match NestedAttrMap::from_nested_meta(attributes_meta) {
        Ok(attrs) => attrs,
        Err(err) => return err.into_compile_error().into(),
    };

    let suite_meta: Meta = parse_quote! { suite = #module_name };
    let suite_nested_meta = NestedMeta::Meta(suite_meta);

    if let Entry::Vacant(entry) = module_attrs.entry("suite".to_owned()) {
        entry.insert(suite_nested_meta);
    };

    if let Some((_, ref mut items)) = test_module.content {
        add_module_imports(items);

        for item in items {
            if let syn::Item::Fn(ref mut f) = item {
                // Check if the function is marked as `connector_test` or `relation_link_test`.
                if let Some(ref mut attr) = f.attrs.iter_mut().find(|attr| match attr.path().get_ident() {
                    Some(ident) => &ident.to_string() == "connector_test" || &ident.to_string() == "relation_link_test",
                    None => false,
                }) {
                    let fn_attrs = match attr.meta {
                        // `connector_test` attribute has no futher attributes.
                        Meta::Path(_) => NestedAttrMap::default(),

                        // `connector_test` attribute has a list of attributes.
                        Meta::List(ref list) => {
                            match NestedMeta::parse_meta_list(list.tokens.clone())
                                .and_then(NestedAttrMap::from_nested_meta)
                            {
                                Ok(list) => list,
                                Err(err) => return err.into_compile_error().into(),
                            }
                        }

                        // Not supported
                        Meta::NameValue(_) => {
                            return syn::Error::new(attr.span(), "Unexpected NameValue list for function attribute.")
                                .into_compile_error()
                                .into();
                        }
                    };

                    let final_attrs = fn_attrs.merge(&module_attrs);

                    // Replace attr.tokens
                    attr.meta = Meta::List(MetaList {
                        path: attr.meta.path().to_owned(),
                        delimiter: MacroDelimiter::Paren(Paren(attr.span())),
                        tokens: quote! { #final_attrs },
                    });
                }
            }
        }
    }

    test_module.into_token_stream().into()
}

fn add_module_imports(items: &mut Vec<Item>) {
    items.reverse();
    items.push(Item::Use(parse_quote! { use super::*; }));
    items.reverse();
}
