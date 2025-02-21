mod datasource;
mod mongodb_renderer;
mod sql_renderer;

use std::sync::LazyLock;

pub use mongodb_renderer::*;
pub use sql_renderer::*;

use crate::{
    connection_string, datamodel_rendering::datasource::DatasourceBuilder, templating, DatamodelFragment, IdFragment,
    M2mFragment, CONFIG,
};
use indoc::indoc;
use itertools::Itertools;
use psl::FeatureMapWithProvider;
use regex::Regex;

/// Test configuration, loaded once at runtime.
static FRAGMENT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"#.*").unwrap());

/// The main trait a datamodel renderer for a connector has to implement.
pub trait DatamodelRenderer {
    fn render(&self, fragment: DatamodelFragment) -> String {
        match fragment {
            DatamodelFragment::Id(id) => self.render_id(id),
            DatamodelFragment::M2m(m2m) => self.render_m2m(m2m),
        }
    }

    fn render_id(&self, id: IdFragment) -> String;
    fn render_m2m(&self, m2m: M2mFragment) -> String;
}

/// Render the complete datamodel with all bells and whistles.
pub fn render_test_datamodel(
    test_database: &str,
    template: String,
    excluded_features: &[&str],
    relation_mode_override: Option<String>,
    db_schemas: &[&str],
    db_extensions: &[&str],
    isolation_level: Option<&'static str>,
) -> String {
    let (tag, version) = CONFIG.test_connector().unwrap();
    let preview_features = render_preview_features(tag.datamodel_provider(), excluded_features);

    let is_multi_schema = !db_schemas.is_empty();

    let datasource = DatasourceBuilder::new("test")
        .provider(tag.datamodel_provider())
        .url(connection_string(
            &CONFIG,
            &version,
            test_database,
            is_multi_schema,
            isolation_level,
        ))
        .relation_mode(relation_mode_override.unwrap_or_else(|| tag.relation_mode().to_string()))
        .schemas_if_not_empty(db_schemas)
        .extensions_if_not_empty(db_extensions)
        .render();

    let datasource_with_generator = format!(
        indoc! {r#"
            {}

            generator client {{
                provider = "prisma-client-js"
                previewFeatures = [{}]
            }}
        "#},
        datasource, preview_features
    );

    let renderer = tag.datamodel_renderer();
    let models = process_template(template, renderer);

    format!("{datasource_with_generator}\n\n{models}")
}

fn process_template(template: String, renderer: Box<dyn DatamodelRenderer>) -> String {
    let mut fragment_defs = vec![];

    for cap in FRAGMENT_RE.captures_iter(&template) {
        let fragment = templating::parse(&cap[0]).unwrap(); // todo error handling
        fragment_defs.push(fragment);
    }

    let preprocessed = FRAGMENT_RE.replace_all(&template, "#{}");

    fragment_defs.into_iter().fold(preprocessed.to_string(), |aggr, next| {
        aggr.replacen("#{}", &renderer.render(next), 1)
    })
}

fn render_preview_features(provider: &str, excluded_features: &[&str]) -> String {
    let excluded_features: Vec<_> = excluded_features.iter().map(|f| format!(r#""{f}""#)).collect();
    let feature_map_with_provider = FeatureMapWithProvider::new(Some(provider));

    feature_map_with_provider
        .active_features()
        .iter()
        .chain(feature_map_with_provider.hidden_features())
        .map(|f| format!(r#""{f}""#))
        .filter(|f| !excluded_features.contains(f))
        .join(", ")
}
