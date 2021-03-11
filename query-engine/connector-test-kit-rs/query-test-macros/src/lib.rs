extern crate proc_macro;

use std::str::FromStr;

use darling::FromMeta;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};

use query_tests_setup::{ConnectorTag, ConnectorTagInterface, ParseError};
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, AttributeArgs, ItemFn, Meta, Path};

#[proc_macro_attribute]
pub fn connector_test(attr: TokenStream, input: TokenStream) -> TokenStream {
    connector_test_impl(attr, input)
}

fn connector_test_impl(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attributes_meta: syn::AttributeArgs = parse_macro_input!(attr as AttributeArgs);
    let args = ConnectorTestArgs::from_list(&attributes_meta);
    let args = match args {
        Ok(args) => args,
        Err(err) => return err.write_errors().into(),
    };

    let test_function = parse_macro_input!(input as ItemFn);
    let ident = test_function.sig.ident.clone();

    let handler = args.schema.unwrap().handler_path;

    let runner_ident = Ident::new(&format!("run_{}", ident.to_string()), Span::call_site());
    let test = quote! {
        #[test]
        fn #runner_ident() {
            let runner = Runner::load();
            let schema = #handler();

            #ident(&runner)
        }

        #test_function
    };

    test.into()
}

#[derive(Debug, FromMeta)]
struct ConnectorTestArgs {
    #[darling(default)]
    schema: Option<SchemaHandler>,

    #[darling(default)]
    only: OnlyConnectorTags,

    #[darling(default)]
    exclude: ExcludeConnectorTags,
}

#[derive(Debug)]
struct SchemaHandler {
    handler_path: Path,
}

impl darling::FromMeta for SchemaHandler {
    fn from_list(items: &[syn::NestedMeta]) -> Result<Self, darling::Error> {
        if items.len() != 1 {
            return Err(darling::Error::unsupported_shape(
                "Expected `schema` to contain exactly one function pointer to a schema handler.",
            )
            .with_span(&Span::call_site()));
        }

        let item = items.first().unwrap();
        match item {
            syn::NestedMeta::Meta(Meta::Path(p)) => Ok(Self {
                // Todo validate signature somehow
                handler_path: p.clone(),
            }),
            x => Err(darling::Error::unsupported_shape(
                "Expected `schema` to be a function pointer to a schema handler function.",
            )
            .with_span(&x.span())),
        }
    }
}

#[derive(Debug, Default)]
struct OnlyConnectorTags {
    tags: Vec<ConnectorTag>,
}

#[derive(Debug, Default)]
struct ExcludeConnectorTags {
    tags: Vec<ConnectorTag>,
}

impl darling::FromMeta for OnlyConnectorTags {
    fn from_list(items: &[syn::NestedMeta]) -> Result<Self, darling::Error> {
        let mut tags: Vec<ConnectorTag> = vec![];

        for item in items {
            match item {
                // syn::NestedMeta::Lit(syn::Lit::Str(s)) => {
                //     todo!("1 {:?}", s)
                // }
                // syn::NestedMeta::Lit(other) => {
                //     todo!("2 {:?}", other)
                // }
                syn::NestedMeta::Meta(meta) => {
                    match meta {
                        // A single variant without version, like `Postgres`.
                        Meta::Path(p) => {
                            let tag = parse_tag_from_path(p)?;
                            tags.push(tag);
                        }
                        Meta::List(l) => {
                            let tag = parse_tag_from_path(&l.path)?;
                            tags.push(tag);

                            let versions = l.nested.iter().filter_map(|meta| match meta {
                                syn::NestedMeta::Lit(literal) => Some(literal),
                                syn::NestedMeta::Meta(_) => None,
                            });

                            for version in versions {
                                let version_str = match version {
                                    syn::Lit::Str(s) => s.value(),
                                    syn::Lit::Char(c) => c.value().to_string(),
                                    syn::Lit::Int(i) => i.to_string(),
                                    syn::Lit::Float(f) => f.to_string(),
                                    x => {
                                        return Err(darling::Error::unexpected_type(
                                            "Versions can be string, char, int and float.",
                                        )
                                        .with_span(&x.span()))
                                    }
                                };

                                let tag = tag.clone();
                                tag.set_version(&version_str);
                                // tags.push();
                            }

                            todo!("{:?}", items);
                            // List(MetaList {
                            //     path: Path {
                            //         leading_colon: None,
                            //         segments: [
                            //             PathSegment {
                            //                 ident: Ident {
                            //                     ident: "Postgres",
                            //                     span: #0 bytes(1849..1857)
                            //                 },
                            //                 arguments: None
                            //             }
                            //         ]
                            //     },
                            //     paren_token: Paren,
                            //     nested: [Lit(Int(LitInt { token: 9 }))]
                            // })
                        }
                        _ => unimplemented!(),
                    }
                }
                x => {
                    return Err(
                        darling::Error::custom("Expected `only` to be a list of `ConnectorTag`.").with_span(&x.span()),
                    )
                }
            }
        }

        Ok(OnlyConnectorTags { tags })
    }
}

fn parse_tag_from_path(path: &Path) -> Result<ConnectorTag, darling::Error> {
    if let Some(ident) = path.get_ident() {
        let name = ident.to_string().to_lowercase();
        let tag = ConnectorTag::from_str(&name).into_darling_error(&path.span())?;

        Ok(tag)
    } else {
        Err(darling::Error::custom(
            "Expected `only` to be a list of valid `ConnectorTag` variants.",
        ))
    }
}

impl darling::FromMeta for ExcludeConnectorTags {
    fn from_list(items: &[syn::NestedMeta]) -> Result<Self, darling::Error> {
        // let mut connectors: Vec<ConnectorTag> = vec![];

        // for item in items {
        //     match item {
        //         // syn::NestedMeta::Lit(syn::Lit::Str(s)) => {
        //         //     todo!("1 {:?}", s)
        //         // }
        //         // syn::NestedMeta::Lit(other) => {
        //         //     todo!("2 {:?}", other)
        //         // }
        //         syn::NestedMeta::Meta(meta) => {
        //             todo!("3 {:?}", meta)
        //         }
        //         x => Err(darling::Error::unsupported_shape(
        //             "Expected `schema` to be a function pointer to a schema handler function.",
        //         )
        //         .with_span(&x.span())),
        //     }
        // }

        todo!()
    }
}

trait IntoDarlingError<T> {
    fn into_darling_error(self, span: &Span) -> std::result::Result<T, darling::Error>;
}

impl<T> IntoDarlingError<T> for std::result::Result<T, ParseError> {
    fn into_darling_error(self, span: &Span) -> std::result::Result<T, darling::Error> {
        self.map_err(|err| darling::Error::custom(&format!("{}.", err.reason)).with_span(span))
    }
}
