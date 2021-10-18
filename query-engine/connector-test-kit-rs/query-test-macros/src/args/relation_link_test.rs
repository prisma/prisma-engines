use super::*;
use crate::IntoDarlingError;
use darling::FromMeta;
use query_tests_setup::{ConnectorTag, RelationField};
use std::convert::TryFrom;
use syn::{Meta, Path};

#[derive(Debug, FromMeta)]
pub struct RelationLinkTestArgs {
    #[darling(default)]
    pub suite: Option<String>,

    #[darling(default)]
    pub only: OnlyConnectorTags,

    pub on_child: OnChild,

    pub on_parent: OnParent,

    #[darling(default)]
    pub id_only: bool,

    #[darling(default)]
    pub exclude: ExcludeConnectorTags,

    #[darling(default)]
    pub capabilities: RunOnlyForCapabilities,
}

impl RelationLinkTestArgs {
    pub fn validate(&self, on_module: bool) -> Result<(), darling::Error> {
        validate_suite(&self.suite, on_module)?;

        Ok(())
    }

    /// Returns all the connectors that the test is valid for.
    pub fn connectors_to_test(&self) -> Vec<ConnectorTag> {
        connectors_to_test(&self.only, &self.exclude)
    }
}

#[derive(Debug)]
pub struct OnChild {
    relation_field: RelationField,
}

impl OnChild {
    pub fn relation_field(&self) -> &RelationField {
        &self.relation_field
    }
}

#[derive(Debug)]
pub struct OnParent {
    relation_field: RelationField,
}

impl OnParent {
    pub fn relation_field(&self) -> &RelationField {
        &self.relation_field
    }
}

impl darling::FromMeta for OnChild {
    fn from_meta(item: &Meta) -> darling::Result<Self> {
        (match *item {
            Meta::NameValue(ref nv) => match nv.lit {
                syn::Lit::Str(ref lit_str) => Ok(OnChild {
                    relation_field: parse_relation_field(lit_str, true)?,
                }),
                _ => Err(darling::Error::custom(
                    "Expected `on_child` to be a String. eg: on_child = \"ToOneOpt\"",
                )),
            },
            _ => Err(darling::Error::custom(
                "Expected `on_child` to be a String. eg: on_child = \"ToOneOpt\"",
            )),
        })
        .map_err(|e| e.with_span(item))
    }
}

impl darling::FromMeta for OnParent {
    fn from_meta(item: &Meta) -> darling::Result<Self> {
        (match *item {
            Meta::NameValue(ref nv) => match nv.lit {
                syn::Lit::Str(ref lit_str) => Ok(OnParent {
                    relation_field: parse_relation_field(lit_str, false)?,
                }),
                _ => Err(darling::Error::custom(
                    "Expected `on_parent` to be a String. eg: on_parent = \"ToOneOpt\"",
                )),
            },
            _ => Err(darling::Error::custom(
                "Expected `on_parent` to be a String. eg: on_parent = \"ToOneOpt\"",
            )),
        })
        .map_err(|e| e.with_span(item))
    }
}

fn parse_relation_field(lit_str: &syn::LitStr, child: bool) -> Result<RelationField, darling::Error> {
    let path: Path = lit_str.parse()?;
    let tag = if let Some(ident) = path.get_ident() {
        let name = ident.to_string();

        Ok(name)
    } else {
        Err(darling::Error::custom(format!(
            "Expected `{}` to be a list of idents (ConnectorTag variants), not paths.",
            if child { "on_child" } else { "on_parent" }
        )))
    }?;

    RelationField::try_from((tag.as_str(), child)).into_darling_error(&lit_str.span())
}
