extern crate proc_macro;

use darling::{FromMeta, ToTokens};
use itertools::Itertools;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use query_tests_setup::{ConnectorTag, ConnectorTagInterface, TestError};
use quote::{quote, TokenStreamExt};
use std::{
    collections::{hash_map, HashMap},
    convert::TryFrom,
    ops::{Deref, DerefMut},
};
use syn::{
    parse_macro_input, parse_quote, spanned::Spanned, AttributeArgs, Item, ItemFn, ItemMod, Meta, NestedMeta, Path,
};

#[derive(Debug, Default)]
struct NestedAttrMap {
    inner: HashMap<String, NestedMeta>,
}

impl Deref for NestedAttrMap {
    type Target = HashMap<String, NestedMeta>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for NestedAttrMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl NestedAttrMap {
    /// Merges this attr map with the incoming one.
    /// Rules:
    /// - If `self` already contains a key, do not overwrite.
    /// - If `self` contains `"only"` or `"exclude"`, then neither of the incoming
    ///   `"only"` or `"exclude"` are merged, because the test overwrites the connectors to test.
    pub fn merge(mut self, other: &Self) -> Self {
        let self_has_connector = self.contains_key("only") || self.contains_key("exclude");

        for (k, v) in other.iter() {
            if (k == "only" || k == "exclude") && !self_has_connector {
                match self.inner.entry(k.clone()) {
                    hash_map::Entry::Occupied(_) => {}
                    hash_map::Entry::Vacant(vacant) => {
                        vacant.insert(v.clone());
                    }
                }
            } else if k != "only" && k != "exclude" {
                match self.inner.entry(k.clone()) {
                    hash_map::Entry::Occupied(_) => {}
                    hash_map::Entry::Vacant(vacant) => {
                        vacant.insert(v.clone());
                    }
                }
            }
        }

        self
    }
}

impl From<&AttributeArgs> for NestedAttrMap {
    fn from(args: &AttributeArgs) -> Self {
        let mut map = HashMap::new();

        for attr in args {
            match attr {
                syn::NestedMeta::Meta(ref meta) => {
                    let ident = meta.path().get_ident().unwrap().to_string();
                    map.insert(ident, attr.clone());
                }
                syn::NestedMeta::Lit(_) => unimplemented!("Unexpected literal encountered in NestedAttrMap parsing."),
            }
        }

        Self { inner: map }
    }
}

impl ToTokens for NestedAttrMap {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let metas: Vec<_> = self.inner.iter().map(|(_, meta)| meta).collect();
        tokens.append_all(quote! { #(#metas),* });
    }
}

#[proc_macro_attribute]
pub fn test_suite(attr: TokenStream, input: TokenStream) -> TokenStream {
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

    if let Err(err) = args.validate(false) {
        return err.write_errors().into();
    };

    let connectors = args.connectors_to_test();
    let handler = args.schema.unwrap().handler_path;

    // Renders the connectors as list to use in the code.
    let connectors = connectors.into_iter().map(quote_connector).fold1(|aggr, next| {
        quote! {
            #aggr, #next
        }
    });

    let mut test_function = parse_macro_input!(input as ItemFn);
    if test_function.sig.asyncness.is_none() {
        return syn::Error::new_spanned(test_function.sig, "connector test functions must be async")
            .to_compile_error()
            .into();
    }

    // The shell function retains the name of the original test definition.
    let test_fn_ident = test_function.sig.ident.clone();

    // Rename original test function to run_<orig_name>.
    let runner_fn_ident = Ident::new(&format!("run_{}", test_fn_ident.to_string()), Span::call_site());
    test_function.sig.ident = runner_fn_ident.clone();

    // Used for logging purposes.
    let test_name = test_fn_ident.to_string();

    // The suite name is the name used as the database for data source rendering.
    let suite_name = args.suite.expect("A test must have a test suite.");

    // The actual test is a shell function that gets the name of the original function,
    // which is then calling `{orig_name}_run` in the end (see `runner_fn_ident`).
    let test = quote! {
        #[test]
        fn #test_fn_ident() {
            let config = &query_tests_setup::CONFIG;
            let enabled_connectors = vec![
                #connectors
            ];
            println!("{:?}", enabled_connectors);

            if !ConnectorTag::should_run(&config, &enabled_connectors) {
                println!("Skipping test '{}', current test connector is not enabled.", #test_name);
                return
            }

            let template = #handler();
            let datamodel = query_tests_setup::render_test_datamodel(config, #suite_name, template);

            query_tests_setup::run_with_tokio(async move {
                let runner = Runner::load(config.runner(), datamodel.clone()).await.unwrap();
                query_tests_setup::setup_project(&datamodel).await.unwrap();
                #runner_fn_ident(&runner).await.unwrap();
            });
        }

        #test_function
    };

    test.into()
}

#[derive(Debug, FromMeta)]
struct ConnectorTestArgs {
    #[darling(default)]
    suite: Option<String>,

    #[darling(default)]
    schema: Option<SchemaHandler>,

    #[darling(default)]
    only: OnlyConnectorTags,

    #[darling(default)]
    exclude: ExcludeConnectorTags,
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
    token_stream: TokenStream,
}

impl OnlyConnectorTags {
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }
}

#[derive(Debug, Default)]
struct ExcludeConnectorTags {
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

fn quote_connector(tag: ConnectorTag) -> proc_macro2::TokenStream {
    let (connector, version) = tag.as_parse_pair();

    match version {
        Some(version) => quote! {
            ConnectorTag::try_from((#connector, Some(#version))).unwrap()
        },
        None => quote! {
            ConnectorTag::try_from(#connector).unwrap()
        },
    }
}

trait IntoDarlingError<T> {
    fn into_darling_error(self, span: &Span) -> std::result::Result<T, darling::Error>;
}

impl<T> IntoDarlingError<T> for std::result::Result<T, TestError> {
    fn into_darling_error(self, span: &Span) -> std::result::Result<T, darling::Error> {
        self.map_err(|err| match err {
            TestError::ParseError(msg) => darling::Error::custom(&format!("Parsing error: {}.", msg)).with_span(span),
            TestError::ConfigError(msg) => {
                darling::Error::custom(&format!("Configuration error: {}.", msg)).with_span(span)
            }
            err => unimplemented!("{:?} not yet handled for test setup compilation", err),
        })
    }
}
