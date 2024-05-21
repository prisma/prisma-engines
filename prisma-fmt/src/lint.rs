use psl::diagnostics::{DatamodelError, DatamodelWarning};

use crate::schema_file_input::SchemaFileInput;

#[derive(serde::Serialize)]
pub struct MiniError {
    file_name: String,
    start: usize,
    end: usize,
    text: String,
    is_warning: bool,
}

pub(crate) fn run(schema: SchemaFileInput) -> String {
    let schema = match schema {
        SchemaFileInput::Single(file) => psl::validate(file.into()),
        SchemaFileInput::Multiple(files) => psl::validate_multi_file(&files),
    };
    let diagnostics = &schema.diagnostics;

    let mut mini_errors: Vec<MiniError> = diagnostics
        .errors()
        .iter()
        .map(|err: &DatamodelError| MiniError {
            file_name: schema.db.file_name(err.span().file_id).to_owned(),
            start: err.span().start,
            end: err.span().end,
            text: err.message().to_string(),
            is_warning: false,
        })
        .collect();

    let mut mini_warnings: Vec<MiniError> = diagnostics
        .warnings()
        .iter()
        .map(|warn: &DatamodelWarning| MiniError {
            file_name: schema.db.file_name(warn.span().file_id).to_owned(),
            start: warn.span().start,
            end: warn.span().end,
            text: warn.message().to_owned(),
            is_warning: true,
        })
        .collect();

    mini_errors.append(&mut mini_warnings);

    print_diagnostics(mini_errors)
}

fn print_diagnostics(diagnostics: Vec<MiniError>) -> String {
    serde_json::to_string(&diagnostics).expect("Failed to render JSON")
}

#[cfg(test)]
mod tests {
    use super::SchemaFileInput;
    use expect_test::expect;
    use indoc::indoc;

    fn lint(schema: SchemaFileInput) -> String {
        let result = super::run(schema);
        let value: serde_json::Value = serde_json::from_str(&result).unwrap();

        serde_json::to_string_pretty(&value).unwrap()
    }

    #[test]
    fn single_deprecated_preview_features_should_give_a_warning() {
        let schema = indoc! {r#"
            datasource db {
              provider = "postgresql"
              url      = env("DATABASE_URL")
            }

            generator client {
              provider = "prisma-client-js"
              previewFeatures = ["createMany"]
            }

            model A {
              id  String   @id
            }
        "#};
        let datamodel = SchemaFileInput::Single(schema.to_string());

        let expected = expect![[r#"
            [
              {
                "file_name": "schema.prisma",
                "start": 149,
                "end": 163,
                "text": "Preview feature \"createMany\" is deprecated. The functionality can be used without specifying it as a preview feature.",
                "is_warning": true
              }
            ]"#]];

        expected.assert_eq(&lint(datamodel));
    }

    #[test]
    fn multi_deprecated_preview_features_should_give_a_warning() {
        let schema1 = indoc! {r#"
            datasource db {
              provider = "postgresql"
              url      = env("DATABASE_URL")
            }

            generator client {
              provider = "prisma-client-js"
              previewFeatures = ["createMany"]
            }
        "#};

        let schema2 = indoc! {r#"
            model A {
              id  String   @id
            }
        "#};

        let datamodel = SchemaFileInput::Multiple(vec![
            ("schema1.prisma".to_string(), schema1.into()),
            ("schema2.prisma".to_string(), schema2.into()),
        ]);

        let expected = expect![[r#"
            [
              {
                "file_name": "schema1.prisma",
                "start": 149,
                "end": 163,
                "text": "Preview feature \"createMany\" is deprecated. The functionality can be used without specifying it as a preview feature.",
                "is_warning": true
              }
            ]"#]];

        expected.assert_eq(&lint(datamodel));
    }
}
