#![allow(clippy::module_inception)]

use crate::common::*;

mod attributes;
mod base;
mod capabilities;
mod common;
mod config;
mod functions;
mod multi_file;
mod parsing;
mod reformat;
mod types;

#[allow(dead_code)]
pub enum Provider {
    Postgres,
    Mysql,
    Sqlite,
    SqlServer,
    Mongo,
    Cockroach,
}

fn with_header(dm: &str, provider: Provider, preview_features: &[&str]) -> String {
    let provider = match provider {
        Provider::Mongo => "mongodb",
        Provider::Postgres => "postgres",
        Provider::Sqlite => "sqlite",
        Provider::Mysql => "mysql",
        Provider::SqlServer => "sqlserver",
        Provider::Cockroach => "cockroachdb",
    };

    let preview_features = if preview_features.is_empty() {
        "".to_string()
    } else {
        format!(
            "previewFeatures = [{}]",
            preview_features
                .iter()
                .map(|f| format!("\"{f}\""))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    let header = formatdoc!(
        r#"
        datasource test {{
          provider = "{provider}"
        }}

        generator client {{
          provider = "prisma-client"
          {preview_features}
        }}
        "#,
    );

    format!("{header}\n{dm}")
}
