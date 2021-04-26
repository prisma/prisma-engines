use proc_macro::{TokenStream, TokenTree};
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, Meta, MetaList, NestedMeta};

#[derive(Default)]
struct TestConnectorAttrs {
    include_tagged: Vec<syn::Path>,
    exclude_tagged: Vec<syn::Path>,
    capabilities: Vec<syn::Path>,
}

impl TestConnectorAttrs {
    fn ingest_meta_list(&mut self, list: MetaList) -> Result<(), syn::Error> {
        let target: &mut Vec<_> = match list.path {
            p if p.is_ident("tags") => &mut self.include_tagged,
            p if p.is_ident("exclude") => &mut self.exclude_tagged,
            p if p.is_ident("capabilities") => &mut self.capabilities,
            p if p.is_ident("logs") => return Ok(()), // TODO
            other => return Err(syn::Error::new_spanned(other, "Unexpected argument")),
        };

        target.reserve(list.nested.len());

        for item in list.nested {
            match item {
                NestedMeta::Meta(Meta::Path(p)) if p.get_ident().is_some() => {
                    target.push(p);
                }
                other => return Err(syn::Error::new_spanned(other, "Unexpected argument")),
            }
        }

        Ok(())
    }
}

pub fn test_connector_impl(attr: TokenStream, input: TokenStream) -> TokenStream {
    use std::fmt::Write;

    let attributes_meta: syn::AttributeArgs = parse_macro_input!(attr as AttributeArgs);

    // We go to some length to avoid parsing the whole function body, so the
    // macro goes faster. We only need to parse at most three tokens:
    // - `async fn #fn_name` or
    // - `fn #fn_name`.
    //
    // This has been measured to make a ~250ms difference on the compile times
    // of migration-engine-tests. This number should only grow over time.
    let mut str_buf = String::with_capacity("async".len());
    let mut fn_tokens = input.into_iter();
    let (original_async_token, mut original_fn_token) = match fn_tokens.next() {
        Some(ident) => {
            write!(str_buf, "{}", ident).unwrap();

            match str_buf.as_str() {
                "async" => (Some(ident), None),
                "fn" => (None, Some(ident)),
                _ => {
                    return syn::Error::new(ident.span().into(), "Bad syntax")
                        .into_compile_error()
                        .into()
                }
            }
        }
        _ => todo!(),
    };

    str_buf.clear();

    // Skip the "fn" token
    if original_async_token.is_some() {
        original_fn_token = fn_tokens.next();
    }

    let (test_function_name, original_fn_name_ident) = match fn_tokens.next() {
        Some(TokenTree::Ident(ident)) => {
            write!(str_buf, "{}", ident).unwrap();

            (syn::Ident::new(&str_buf, ident.span().into()), ident)
        }
        _ => todo!(),
    };

    let mut attrs = TestConnectorAttrs::default();

    for meta in attributes_meta {
        match meta {
            NestedMeta::Meta(Meta::List(list)) => {
                if let Err(err) = attrs.ingest_meta_list(list) {
                    return err.to_compile_error().into();
                }
            }
            other => {
                return syn::Error::new_spanned(other, "Unexpected argument")
                    .into_compile_error()
                    .into()
            }
        }
    }

    let include_tagged = &attrs.include_tagged;
    let exclude_tagged = &attrs.exclude_tagged;
    let capabilities = &attrs.capabilities;

    let tokens = if original_async_token.is_some() {
        quote! {
            mod #test_function_name {
                #[test]
                fn test() {
                    use super::*;

                    let args = test_setup::TestApiArgs::new(#str_buf);

                    if test_setup::should_skip_test(
                        &args,
                        BitFlags::empty() #(| Tags::#include_tagged)*,
                        BitFlags::empty() #(| Tags::#exclude_tagged)*,
                        BitFlags::empty() #(| Capabilities::#capabilities)*,
                    ) { return }

                    test_setup::runtime::run_with_tokio(async {
                        let api = TestApi::new(args).await;
                        super::#test_function_name(&api).await
                    }).unwrap()
                }
            }
        }
    } else {
        quote! {
            mod #test_function_name {
                #[test]
                fn test() {
                    use super::*;

                    let args = test_setup::TestApiArgs::new(#str_buf);

                    if test_setup::should_skip_test(
                        &args,
                        BitFlags::empty() #(| Tags::#include_tagged)*,
                        BitFlags::empty() #(| Tags::#exclude_tagged)*,
                        BitFlags::empty() #(| Capabilities::#capabilities)*,
                    ) { return }

                    #test_function_name ( TestApi::new(args) )
                }
            }
        }
    };

    let mut token_stream: TokenStream = tokens.into();
    let rest = original_async_token
        .into_iter()
        .chain(original_fn_token.into_iter())
        .chain(std::iter::once(TokenTree::Ident(original_fn_name_ident)))
        .chain(fn_tokens);

    token_stream.extend(rest);

    token_stream
}
