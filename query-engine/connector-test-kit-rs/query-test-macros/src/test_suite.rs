use crate::{attr_map::NestedAttrMap, ConnectorTestArgs};
use darling::{FromMeta, ToTokens};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, AttributeArgs, Item, ItemMod, Meta, NestedMeta};

pub fn test_suite_impl(attr: TokenStream, input: TokenStream) -> TokenStream {
    // Validate input by simply parsing it, which will point out invalid fields and connector names etc.
    let attributes_meta: syn::AttributeArgs = parse_macro_input!(attr as AttributeArgs);
    let args = ConnectorTestArgs::from_list(&attributes_meta);
    let args = match args {
        Ok(args) => args,
        Err(err) => return err.write_errors().into(),
    };

    if let Err(err) = args.validate(true) {
        return err.write_errors().into();
    };
    // end validation

    let mut test_module = parse_macro_input!(input as ItemMod);
    let module_name = test_module.ident.clone().to_string();
    let mut module_attrs = NestedAttrMap::from(&attributes_meta);

    let suite_meta: Meta = parse_quote! { suite = #module_name };
    let suite_nested_meta = NestedMeta::from(suite_meta);
    module_attrs.insert("suite".to_owned(), suite_nested_meta);

    if let Some((_, ref mut items)) = test_module.content {
        add_module_imports(items);

        for item in items {
            if let syn::Item::Fn(ref mut f) = item {
                // Check if the function is marked as `connector_test`.
                if let Some(ref mut attr) = f.attrs.iter_mut().find(|attr| match attr.path.get_ident() {
                    Some(ident) => &ident.to_string() == "connector_test",
                    None => false,
                }) {
                    let meta = attr.parse_meta().expect("Invalid attribute meta.");
                    let fn_attrs = match meta {
                        // `connector_test` attribute has no futher attributes.
                        Meta::Path(_) => NestedAttrMap::default(),

                        // `connector_test` attribute has a list of attributes.
                        Meta::List(l) => NestedAttrMap::from(&l.nested.clone().into_iter().collect::<Vec<_>>()),

                        // Not supported
                        Meta::NameValue(_) => unimplemented!("Unexpected NameValue list for function attribute."),
                    };

                    let final_attrs = fn_attrs.merge(&module_attrs);

                    // Replace attr.tokens
                    attr.tokens = quote! { (#final_attrs) };
                }
            }
        }
    }

    test_module.into_token_stream().into()
}

fn add_module_imports(items: &mut Vec<Item>) {
    items.reverse();
    items.push(Item::Use(parse_quote! { use super::*; }));
    items.push(Item::Use(parse_quote! { use query_tests_setup::*; }));
    items.push(Item::Use(parse_quote! { use std::convert::TryFrom; }));
    items.reverse();
}
