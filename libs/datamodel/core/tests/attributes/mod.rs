use indoc::formatdoc;
use itertools::Itertools;

mod arg_parsing;
mod builtin_attributes;
mod constraint_names;
mod constraint_names_negative;
mod constraint_names_positive;
mod default_negative;
mod default_positive;
mod id_negative;
mod id_positive;
mod ignore_negative;
mod ignore_positive;
mod index_negative;
mod index_positive;
mod map_negative;
mod map_positive;
mod relations;
mod unique_negative;
mod unique_positive;
mod updated_at_negative;
mod updated_at_positive;

#[allow(dead_code)]
pub enum Provider {
    Postgres,
    Mysql,
    Sqlite,
    SqlServer,
    Mongo,
}

fn with_header(dm: &str, provider: Provider, preview_features: &[&str]) -> String {
    let (provider, url) = match provider {
        Provider::Mongo => ("mongodb", "mongo"),
        Provider::Postgres => ("postgres", "postgresql"),
        Provider::Sqlite => ("sqlite", "file"),
        Provider::Mysql => ("mysql", "mysql"),
        Provider::SqlServer => ("sqlserver", "sqlserver"),
    };

    let preview_features = if preview_features.is_empty() {
        "".to_string()
    } else {
        format!(
            "previewFeatures = [{}]",
            preview_features.iter().map(|f| format!("\"{}\"", f)).join(", ")
        )
    };

    let header = formatdoc!(
        r#"
        datasource test {{
          provider = "{}"
          url = "{}://..."
        }}
        
        generator client {{
          provider = "prisma-client-js"
          {}
        }}
        "#,
        provider,
        url,
        preview_features
    );

    format!("{}\n{}", header, dm)
}
