extern crate proc_macro;

use darling::{ast::NestedMeta, FromMeta};
use proc_macro::TokenStream;
use proc_macro2::{Delimiter, TokenTree};
use quote::quote;
use syn::{Lit, LitStr, Meta, Signature};

#[proc_macro_attribute]
pub fn test_connector(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = proc_macro2::TokenStream::from(input);

    let attrs = match NestedMeta::parse_meta_list(args.into()) {
        Ok(v) => match TestConnectorAttrs::from_list(&v) {
            Ok(args) => args,
            Err(e) => return e.write_errors().into(),
        },
        Err(e) => return e.into_compile_error().into(),
    };

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
    let include_tagged = &attrs.include_tagged.0;
    let exclude_tagged = &attrs.exclude_tagged.0;
    let capabilities = &attrs.capabilities.0;
    let preview_features = &attrs.preview_features.0;
    let namespaces = &attrs.namespaces.0;

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

#[derive(FromMeta)]
struct TestConnectorAttrs {
    #[darling(default, rename = "tags")]
    include_tagged: PathList,
    #[darling(default, rename = "exclude")]
    exclude_tagged: PathList,
    #[darling(default)]
    capabilities: PathList,
    #[darling(default)]
    preview_features: LitStrList,
    #[darling(default)]
    namespaces: LitStrList,
    #[darling(default, rename = "ignore")]
    ignore_reason: Option<LitStr>,
}

#[derive(Default)]
struct PathList(Vec<syn::Path>);

impl FromMeta for PathList {
    fn from_list(items: &[NestedMeta]) -> darling::Result<Self> {
        Ok(Self(
            items
                .iter()
                .map(|meta| match meta {
                    NestedMeta::Meta(Meta::Path(path)) => Ok(path.clone()),
                    _ => Err(darling::Error::custom("Unexpected argument").with_span(meta)),
                })
                .collect::<darling::Result<Vec<_>>>()?,
        ))
    }
}

#[derive(Default)]
struct LitStrList(Vec<syn::LitStr>);

impl FromMeta for LitStrList {
    fn from_list(items: &[NestedMeta]) -> darling::Result<Self> {
        Ok(Self(
            items
                .iter()
                .map(|meta| match meta {
                    NestedMeta::Lit(Lit::Str(lit)) => Ok(lit.clone()),
                    _ => Err(darling::Error::custom("Unexpected argument").with_span(meta)),
                })
                .collect::<darling::Result<Vec<_>>>()?,
        ))
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
