extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{Delimiter, TokenTree};
use quote::quote;
use darling::ast::NestedMeta;
use syn::{Expr, Lit, LitStr, Meta, MetaList, MetaNameValue, ExprLit, Signature};

#[proc_macro_attribute]
pub fn test_connector(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attributes_meta: Vec<NestedMeta> = match NestedMeta::parse_meta_list(attr.into()) {
        Ok(v) => v,
        Err(e) => { return TokenStream::from(darling::Error::from(e).write_errors()); }
    };
    let input = proc_macro2::TokenStream::from(input);

    // First the attributes
    let mut attrs = TestConnectorAttrs::default();

    for meta in attributes_meta {
        match meta {
            NestedMeta::Meta(Meta::List(list)) => {
                if let Err(err) = attrs.ingest_meta_list(list) {
                    return err.to_compile_error().into();
                }
            }
            NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                eq_token: _,
                path,
                value: Expr::Lit(ExprLit { lit: Lit::Str(litstr), .. }),
            })) if path.is_ident("ignore") => attrs.ignore_reason = Some(litstr),
            other => {
                return syn::Error::new_spanned(other, "Unexpected argument")
                    .into_compile_error()
                    .into()
            }
        }
    }

    // Then the function item
    // We take advantage of the function body being the last token tree (surrounded by braces).
    let (sig, body): (Signature, proc_macro2::TokenStream) = {
        let sig_tokens = input
            .clone()
            .into_iter()
            .take_while(|t| !matches!(t, TokenTree::Group(g) if matches!(g.delimiter(), Delimiter::Brace)))
            .collect();

        let body = input.into_iter().last().expect("Failed to find function body");

        match syn::parse2(sig_tokens) {
            Ok(sig) => (sig, body.into()),
            Err(err) => return err.into_compile_error().into(),
        }
    };

    // Generate the final function
    let include_tagged = &attrs.include_tagged;
    let exclude_tagged = &attrs.exclude_tagged;
    let capabilities = &attrs.capabilities;
    let preview_features = &attrs.preview_features;
    let namespaces = &attrs.namespaces;

    let test_function_name = &sig.ident;
    let test_function_name_lit = sig.ident.to_string();
    let (arg_name, arg_type) = match extract_api_arg(&sig) {
        Ok(args) => args,
        Err(err) => return err.to_compile_error().into(),
    };
    let ignore_attr = attrs.ignore_reason.map(|reason| quote!(#[ignore = #reason]));

    let tokens = if sig.asyncness.is_some() {
        let (return_ty, unwrap) = match sig.output {
            syn::ReturnType::Default => (quote!(()), quote!()),
            syn::ReturnType::Type(_, ref ty) => (quote!(#ty), quote!(.unwrap())),
        };

        quote! {
            #[test]
            #ignore_attr
            fn #test_function_name() {
                let args = test_setup::TestApiArgs::new(#test_function_name_lit, &[#(#preview_features,)*], &[#(#namespaces,)*]);

                if test_setup::should_skip_test(
                    BitFlags::empty() #(| Tags::#include_tagged)*,
                    BitFlags::empty() #(| Tags::#exclude_tagged)*,
                    BitFlags::empty() #(| Capabilities::#capabilities)*,
                ) { return }

                test_setup::runtime::run_with_thread_local_runtime::<#return_ty>(async {
                    let #arg_name = &mut #arg_type::new(args).await;

                    #body

                })#unwrap;
            }
        }
    } else {
        quote! {
            #[test]
            #ignore_attr
            fn #test_function_name() {
                let args = test_setup::TestApiArgs::new(#test_function_name_lit, &[#(#preview_features,)*], &[#(#namespaces,)*]);

                if test_setup::should_skip_test(
                    BitFlags::empty() #(| Tags::#include_tagged)*,
                    BitFlags::empty() #(| Tags::#exclude_tagged)*,
                    BitFlags::empty() #(| Capabilities::#capabilities)*,
                ) { return }

                #[allow(all)]
                let mut #arg_name = #arg_type::new(args);

                #body
            }
        }
    };

    tokens.into()
}

#[derive(Default)]
struct TestConnectorAttrs {
    include_tagged: Vec<syn::Path>,
    exclude_tagged: Vec<syn::Path>,
    capabilities: Vec<syn::Path>,
    preview_features: Vec<syn::LitStr>,
    namespaces: Vec<syn::LitStr>,
    ignore_reason: Option<LitStr>,
}

impl TestConnectorAttrs {
    fn ingest_meta_list(&mut self, list: MetaList) -> Result<(), syn::Error> {
        let nested = NestedMeta::parse_meta_list(list.tokens)?;

        let target: &mut Vec<_> = match list.path {
            p if p.is_ident("tags") => &mut self.include_tagged,
            p if p.is_ident("exclude") => &mut self.exclude_tagged,
            p if p.is_ident("capabilities") => &mut self.capabilities,
            p if p.is_ident("preview_features") => {
                self.preview_features.reserve(nested.len());

                for item in nested {
                    match item {
                        NestedMeta::Lit(Lit::Str(s)) => self.preview_features.push(s),
                        other => return Err(syn::Error::new_spanned(other, "Unexpected argument")),
                    }
                }

                return Ok(());
            }
            p if p.is_ident("namespaces") => {
                self.namespaces.reserve(nested.len());

                for item in nested {
                    match item {
                        NestedMeta::Lit(Lit::Str(s)) => self.namespaces.push(s),
                        other => return Err(syn::Error::new_spanned(other, "Unexpected argument")),
                    }
                }

                return Ok(());
            }
            other => return Err(syn::Error::new_spanned(other, "Unexpected argument")),
        };

        target.reserve(nested.len());

        for item in nested {
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

fn extract_api_arg(sig: &Signature) -> Result<(&syn::Ident, &syn::Ident), syn::Error> {
    use syn::spanned::Spanned;

    let err = |span| {
        Err(syn::Error::new(
            span,
            format!(
                "Unsupported syntax. Arguments to test functions should be of the form `fn test_fn(api: {}TestApi)`",
                if sig.asyncness.is_some() { "&mut " } else { "" }
            ),
        ))
    };

    match (sig.inputs.first(), sig.inputs.len()) {
        (Some(syn::FnArg::Typed(pattype)), 1) => {
            let arg_name = match pattype.pat.as_ref() {
                syn::Pat::Ident(ident) => &ident.ident,
                other => return err(other.span()),
            };

            let arg_type = match pattype.ty.as_ref() {
                syn::Type::Reference(syn::TypeReference {
                    mutability: Some(_),
                    elem,
                    ..
                }) if sig.asyncness.is_some() => match elem.as_ref() {
                    syn::Type::Path(ident) => ident.path.get_ident().unwrap(),
                    other => return err(other.span()),
                },
                syn::Type::Path(ident) => ident.path.get_ident().unwrap(),
                other => return err(other.span()),
            };

            Ok((arg_name, arg_type))
        }
        (_, n) => Err(syn::Error::new_spanned(
            &sig.inputs,
            format!("Test functions should take one argument, not {n}"),
        )),
    }
}
