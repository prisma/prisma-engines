mod mongodb_renderer;
mod sql_renderer;

pub use mongodb_renderer::*;
pub use sql_renderer::*;

use crate::{templating, ConnectorTagInterface, DatamodelFragment, IdFragment, M2mFragment, TestConfig};
use datamodel::common::preview_features::GENERATOR;
use indoc::indoc;
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Test configuration, loaded once at runtime.
    static ref FRAGMENT_RE: Regex = Regex::new(r"#.*").unwrap();
}

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
    config: &TestConfig,
    test_database: &str,
    template: String,
    excluded_features: &[&str],
    referential_integrity_override: Option<String>,
    db_schemas: &[&str],
) -> String {
    let tag = config.test_connector_tag().unwrap();
    let preview_features = render_preview_features(excluded_features);

    let is_multi_schema = !db_schemas.is_empty();

    let schema_def = if is_multi_schema {
        format!("schemas = {:?}", db_schemas)
    } else {
        String::default()
    };

    let datasource_with_generator = format!(
        indoc! {r#"
            datasource test {{
                provider = "{}"
                url = "{}"
                referentialIntegrity = "{}"
                {}
            }}

            generator client {{
                provider = "prisma-client-js"
                previewFeatures = [{}]
            }}
        "#},
        tag.datamodel_provider(),
        tag.connection_string(test_database, config.is_ci(), is_multi_schema),
        referential_integrity_override.unwrap_or_else(|| tag.referential_integrity().to_string()),
        schema_def,
        preview_features
    );

    let renderer = tag.datamodel_renderer();
    let models = process_template(template, renderer);

    format!("{}\n\n{}", datasource_with_generator, models)
}

fn process_template(template: String, renderer: Box<dyn DatamodelRenderer>) -> String {
    let mut fragment_defs = vec![];

    for cap in FRAGMENT_RE.captures_iter(&template) {
        let fragment = templating::parse(&cap[0]).unwrap(); // todo error handling
        fragment_defs.push(fragment);
    }

    let preprocessed = FRAGMENT_RE.replace_all(&template, "#{}").to_owned();

    fragment_defs.into_iter().fold(preprocessed.to_string(), |aggr, next| {
        aggr.replacen("#{}", &renderer.render(next), 1)
    })
}

fn render_preview_features(excluded_features: &[&str]) -> String {
    let excluded_features: Vec<_> = excluded_features.iter().map(|f| format!(r#""{}""#, f)).collect();

    GENERATOR
        .active_features()
        .iter()
        .chain(GENERATOR.hidden_features())
        .map(|f| format!(r#""{}""#, f))
        .filter(|f| !excluded_features.contains(f))
        .join(", ")
}
