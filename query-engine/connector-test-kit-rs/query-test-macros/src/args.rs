use crate::IntoDarlingError;
use darling::FromMeta;
use proc_macro::TokenStream;
use proc_macro2::Span;
use query_tests_setup::ConnectorTag;
use quote::quote;
use std::convert::TryFrom;
use syn::{spanned::Spanned, Meta, Path};

#[derive(Debug, FromMeta)]
pub struct ConnectorTestArgs {
    #[darling(default)]
    pub suite: Option<String>,

    #[darling(default)]
    pub schema: Option<SchemaHandler>,

    #[darling(default)]
    pub only: OnlyConnectorTags,

    #[darling(default)]
    pub exclude: ExcludeConnectorTags,
}

impl ConnectorTestArgs {
    pub fn validate(&self, on_module: bool) -> Result<(), darling::Error> {
        if !self.only.is_empty() && !self.exclude.is_empty() && !on_module {
            return Err(darling::Error::custom(
                "Only one of `only` and `exclude` can be specified for a connector test.",
            ));
        }

        if self.schema.is_none() && !on_module {
            return Err(darling::Error::custom(
                "A schema annotation on either the test mod (#[test_suite(schema(handler))]) or the test (schema(handler)) is required.",
            ));
        }

        if self.suite.is_none() && !on_module {
            return Err(darling::Error::custom(
                "A test suite name annotation on either the test mod (#[test_suite]) or the test (suite = \"name\") is required.",
            ));
        }

        Ok(())
    }

    /// Returns all the connectors that the test is valid for.
    pub fn connectors_to_test(&self) -> Vec<ConnectorTag> {
        if !self.only.is_empty() {
            self.only.tags.clone()
        } else if !self.exclude.is_empty() {
            let all = ConnectorTag::all();
            let exclude = self.exclude.tags();

            all.into_iter().filter(|tag| !exclude.contains(tag)).collect()
        } else {
            ConnectorTag::all()
        }
    }
}

#[derive(Debug)]
pub struct SchemaHandler {
    pub handler_path: Path,
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
pub struct OnlyConnectorTags {
    tags: Vec<ConnectorTag>,
    token_stream: TokenStream,
}

impl OnlyConnectorTags {
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }
}

#[derive(Debug, Default)]
pub struct ExcludeConnectorTags {
    tags: Vec<ConnectorTag>,
}

impl ExcludeConnectorTags {
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }

    pub fn tags(&self) -> &[ConnectorTag] {
        &self.tags
    }
}

impl darling::FromMeta for OnlyConnectorTags {
    fn from_list(items: &[syn::NestedMeta]) -> Result<Self, darling::Error> {
        let token_stream = quote! { #(#items),* }.into();
        let tags = tags_from_list(items)?;

        Ok(OnlyConnectorTags { tags, token_stream })
    }
}

impl darling::FromMeta for ExcludeConnectorTags {
    fn from_list(items: &[syn::NestedMeta]) -> Result<Self, darling::Error> {
        let tags = tags_from_list(items)?;
        Ok(ExcludeConnectorTags { tags })
    }
}

fn tags_from_list(items: &[syn::NestedMeta]) -> Result<Vec<ConnectorTag>, darling::Error> {
    if items.is_empty() {
        return Err(darling::Error::custom("At least one connector tag is required."));
    }

    let mut tags: Vec<ConnectorTag> = vec![];

    for item in items {
        match item {
            syn::NestedMeta::Meta(meta) => {
                match meta {
                    // A single variant without version, like `Postgres`.
                    Meta::Path(p) => {
                        let tag = tag_string_from_path(p)?;
                        tags.push(ConnectorTag::try_from(tag.as_str()).into_darling_error(&p.span())?);
                    }
                    Meta::List(l) => {
                        let tag = tag_string_from_path(&l.path)?;
                        for meta in l.nested.iter() {
                            match meta {
                                syn::NestedMeta::Lit(literal) => {
                                    let version_str = match literal {
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

                                    tags.push(
                                        ConnectorTag::try_from((tag.as_str(), Some(version_str.as_str())))
                                            .into_darling_error(&l.span())?,
                                    );
                                }
                                syn::NestedMeta::Meta(meta) => {
                                    return Err(darling::Error::unexpected_type(
                                        "Versions can only be literals (string, char, int and float).",
                                    )
                                    .with_span(&meta.span()));
                                }
                            }
                        }
                    }
                    _ => unimplemented!(),
                }
            }
            x => {
                return Err(
                    darling::Error::custom("Expected `only` or `exclude` to be a list of `ConnectorTag`.")
                        .with_span(&x.span()),
                )
            }
        }
    }

    Ok(tags)
}

fn tag_string_from_path(path: &Path) -> Result<String, darling::Error> {
    if let Some(ident) = path.get_ident() {
        let name = ident.to_string();

        Ok(name)
    } else {
        Err(darling::Error::custom(
            "Expected `only` to be a list of idents (ConnectorTag variants), not paths.",
        ))
    }
}
