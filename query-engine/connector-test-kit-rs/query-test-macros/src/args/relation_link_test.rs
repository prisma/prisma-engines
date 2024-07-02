use super::{connector_test::*, *};
use darling::FromMeta;
use syn::{Meta, Path};

#[derive(Debug)]
pub(crate) struct RelationField(String, bool);

impl darling::ToTokens for RelationField {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let s = &self.0;
        let b = &self.1;
        tokens.extend(quote::quote! { &::query_tests_setup::RelationField::try_from((#s, #b)).unwrap() })
    }
}

#[derive(Debug, FromMeta)]
pub(crate) struct RelationLinkTestArgs {
    #[darling(default)]
    pub suite: Option<String>,

    #[darling(default)]
    pub only: ConnectorTags,

    pub(crate) on_child: OnChild,

    pub(crate) on_parent: OnParent,

    #[darling(default)]
    pub(crate) id_only: bool,

    #[darling(default)]
    pub(crate) exclude: ConnectorTags,

    #[darling(default)]
    pub capabilities: RunOnlyForCapabilities,
}

impl RelationLinkTestArgs {
    pub fn validate(&self, on_module: bool) -> Result<(), darling::Error> {
        utils::validate_suite(&self.suite, on_module)
    }
}

#[derive(Debug)]
pub(crate) struct OnChild {
    pub(crate) relation_field: RelationField,
}

#[derive(Debug)]
pub(crate) struct OnParent {
    pub(crate) relation_field: RelationField,
}

impl darling::FromMeta for OnChild {
    fn from_meta(item: &Meta) -> darling::Result<Self> {
        (match *item {
            Meta::NameValue(ref nv) => match nv.value {
                syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(ref lit_str), .. }) => Ok(OnChild {
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
            Meta::NameValue(ref nv) => match nv.value {
                syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(ref lit_str), .. }) => Ok(OnParent {
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

    Ok(RelationField(tag, child))
}
