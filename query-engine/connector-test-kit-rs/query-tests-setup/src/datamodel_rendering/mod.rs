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
pub fn render_test_datamodel(config: &TestConfig, test_database: &str, template: String) -> String {
    let tag = config.test_connector_tag().unwrap();
    let all_features = GENERATOR
        .active_features()
        .iter()
        .chain(GENERATOR.hidden_features())
        .map(|f| format!(r#""{}""#, f.to_string()))
        .join(", ");

    let datasource_with_generator = format!(
        indoc! {r#"
            datasource test {{
                provider = "{}"
                url = "{}"
                referentialIntegrity = "{}"
            }}

            generator client {{
                provider = "prisma-client-js"
                previewFeatures = [{}]
            }}
        "#},
        tag.datamodel_provider(),
        tag.connection_string(test_database, config.is_ci()),
        tag.referential_integrity(),
        all_features
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
