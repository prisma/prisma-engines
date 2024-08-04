use enumflags2::BitFlags;
use names::Generator;
use once_cell::sync::Lazy;
use psl::PreviewFeature;
use std::io::Write as _;

pub static CONN_STR: Lazy<String> = Lazy::new(|| match std::env::var("TEST_DATABASE_URL") {
    Ok(url) => url,
    Err(_) => {
        let stderr = std::io::stderr();

        let mut sink = stderr.lock();
        sink.write_all(b"Please set TEST_DATABASE_URL env var pointing to a MongoDB instance.")
            .unwrap();
        sink.write_all(b"\n").unwrap();

        std::process::exit(1)
    }
});

pub(crate) fn generate_database_name() -> String {
    let mut names = Generator::default();

    names.next().unwrap().replace('-', "")
}

pub(crate) fn get_connection_string(database_name: &str) -> String {
    let mut connection_string: url::Url = CONN_STR.parse().unwrap();
    connection_string.set_path(&format!(
        "/{}{}",
        database_name,
        connection_string.path().trim_start_matches('/')
    ));

    connection_string.to_string()
}

pub(crate) fn datasource_block_string() -> String {
    indoc::formatdoc!(
        r#"
          datasource db {{
            provider = "mongodb"
            url      = "env(TEST_DATABASE_URL)"
          }}
      "#
    )
}

pub(crate) fn generator_block_string(features: BitFlags<PreviewFeature>) -> String {
    let features = features
        .iter()
        .map(|f| format!("\"{f}\""))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        r#"
          generator js {{
            provider        = "prisma-client-js"
            previewFeatures = [{}]
          }}
      "#,
        features,
    )
}

pub(crate) fn config_block_string(features: BitFlags<PreviewFeature>) -> String {
    format!("{}\n{}", generator_block_string(features), datasource_block_string())
}

#[track_caller]
pub(crate) fn parse_datamodels(datamodels: &[(&str, String)]) -> psl::ValidatedSchema {
    let datamodels: Vec<_> = datamodels
        .iter()
        .map(|(file_name, dm)| (file_name.to_string(), psl::SourceFile::from(dm)))
        .collect();

    psl::validate_multi_file(&datamodels)
}
