use super::*;
use darling::{FromMeta, ToTokens};
use proc_macro2::Span;
use quote::quote;
use syn::{spanned::Spanned, Ident, Meta, Path};

type ConnectorTag = (String, Option<String>);

#[derive(Debug, FromMeta)]
pub struct ConnectorTestArgs {
    #[darling(default)]
    pub suite: Option<String>,

    #[darling(default)]
    pub schema: Option<SchemaHandler>,

    #[darling(default)]
    pub only: ConnectorTags,

    #[darling(default)]
    pub exclude: ConnectorTags,

    #[darling(default)]
    pub exclude_features: ExcludeFeatures,

    #[darling(default)]
    pub capabilities: RunOnlyForCapabilities,

    // #[deprecated(since = "4.5.0", note = "Use `relation_mode` instead")]
    #[darling(default)]
    pub referential_integrity: Option<RelationMode>,

    #[darling(default)]
    pub relation_mode: Option<RelationMode>,

    #[darling(default)]
    pub db_schemas: DbSchemas,
}

impl ConnectorTestArgs {
    pub fn validate(&self, on_module: bool) -> Result<(), darling::Error> {
        utils::validate_suite(&self.suite, on_module)?;

        if self.schema.is_none() && !on_module {
            return Err(darling::Error::custom(
                "A schema annotation on either the test mod (#[test_suite(schema(handler))]) or the test (schema(handler)) is required.",
            ));
        }

        Ok(())
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum RelationMode {
    ForeignKeys,
    Prisma,
}

impl darling::FromMeta for RelationMode {
    fn from_string(value: &str) -> darling::Result<Self> {
        match value.to_lowercase().as_str() {
            "prisma" => Ok(Self::Prisma),
            "foreignkeys" => Ok(Self::ForeignKeys),
            _ => Err(darling::Error::custom(format!("Invalid value: {value}"))),
        }
    }
}

impl ToString for RelationMode {
    fn to_string(&self) -> String {
        match self {
            Self::Prisma => "prisma".to_string(),
            Self::ForeignKeys => "foreignKeys".to_string(),
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
pub struct ConnectorTags {
    tags: Vec<ConnectorTag>,
}

impl ToTokens for ConnectorTags {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(self.tags.iter().map(|(connector, version)| match version {
            Some(v) => quote!((#connector, Some(#v)),),
            None => quote!((#connector, None),),
        }))
    }
}

#[derive(Debug, Default)]
pub struct ExcludeFeatures {
    features: Vec<String>,
}

impl ExcludeFeatures {
    pub fn features(&self) -> &[String] {
        self.features.as_ref()
    }
}

#[derive(Debug, Default)]
pub struct DbSchemas {
    db_schemas: Vec<String>,
}

impl DbSchemas {
    pub fn schemas(&self) -> &[String] {
        self.db_schemas.as_ref()
    }
}

impl darling::FromMeta for DbSchemas {
    fn from_list(items: &[syn::NestedMeta]) -> Result<Self, darling::Error> {
        let db_schemas = strings_to_list("DbSchemas", items)?;
        Ok(DbSchemas { db_schemas })
    }
}

impl darling::FromMeta for ExcludeFeatures {
    fn from_list(items: &[syn::NestedMeta]) -> Result<Self, darling::Error> {
        let features = strings_to_list("Preview Features", items)?;

        Ok(ExcludeFeatures { features })
    }
}

fn strings_to_list(name: &str, items: &[syn::NestedMeta]) -> Result<Vec<String>, darling::Error> {
    let error = format!("{name} can only be string literals.");
    items
        .iter()
        .map(|i| match i {
            syn::NestedMeta::Meta(m) => Err(darling::Error::unexpected_type(error.as_str()).with_span(&m.span())),
            syn::NestedMeta::Lit(l) => match l {
                syn::Lit::Str(s) => Ok(s.value()),
                _ => Err(darling::Error::unexpected_type(&error).with_span(&l.span())),
            },
        })
        .collect::<Result<Vec<_>, _>>()
}

impl darling::FromMeta for ConnectorTags {
    fn from_list(items: &[syn::NestedMeta]) -> Result<Self, darling::Error> {
        let tags = tags_from_list(items)?;
        Ok(ConnectorTags { tags })
    }
}

fn tags_from_list(items: &[syn::NestedMeta]) -> Result<Vec<ConnectorTag>, darling::Error> {
    if items.is_empty() {
        return Err(darling::Error::custom("At least one connector tag is required."));
    }

    let mut tags: Vec<ConnectorTag> = Vec::with_capacity(items.len());

    for item in items {
        match item {
            syn::NestedMeta::Meta(meta) => {
                match meta {
                    // A single variant without version, like `Postgres`.
                    Meta::Path(p) => {
                        let tag = tag_string_from_path(p)?;
                        tags.push((tag, None));
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

                                    tags.push((tag.clone(), Some(version_str)));
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

#[derive(Debug, Default)]
pub struct RunOnlyForCapabilities {
    pub idents: Vec<Ident>,
}

impl darling::FromMeta for RunOnlyForCapabilities {
    fn from_list(items: &[syn::NestedMeta]) -> Result<Self, darling::Error> {
        if items.is_empty() {
            return Err(darling::Error::custom(
                "When specifying capabilities to run for, at least one needs to be given.",
            ));
        }

        let mut idents: Vec<Ident> = vec![];

        for item in items {
            match item {
                syn::NestedMeta::Meta(meta) => {
                    match meta {
                        // A single variant without version, like `Postgres`.
                        Meta::Path(p) => match p.get_ident() {
                            Some(ident) => idents.push(ident.clone()),
                            None => {
                                return Err(darling::Error::unexpected_type("Invalid identifier").with_span(&p.span()))
                            }
                        },
                        x => return Err(darling::Error::unexpected_type("Expected identifiers").with_span(&x.span())),
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

        Ok(Self { idents })
    }
}
