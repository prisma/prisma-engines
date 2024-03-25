use lsp_types::{Diagnostic, DiagnosticSeverity};
use once_cell::sync::Lazy;
use prisma_fmt::offset_to_position;
use psl::SourceFile;
use std::{fmt::Write as _, io::Write as _, sync::Arc};

const SCENARIOS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/code_actions/scenarios");
static UPDATE_EXPECT: Lazy<bool> = Lazy::new(|| std::env::var("UPDATE_EXPECT").is_ok());

fn parse_schema_diagnostics(file: impl Into<SourceFile>) -> Option<Vec<Diagnostic>> {
    let schema = psl::validate(file.into());

    match (schema.diagnostics.warnings(), schema.diagnostics.errors()) {
        ([], []) => None,
        (warnings, errors) => {
            let mut diagnostics = Vec::new();
            for warn in warnings.iter() {
                diagnostics.push(Diagnostic {
                    severity: Some(DiagnosticSeverity::WARNING),
                    message: warn.message().to_owned(),
                    range: lsp_types::Range {
                        start: offset_to_position(warn.span().start(), schema.db.source_assert_single()),
                        end: offset_to_position(warn.span().end(), schema.db.source_assert_single()),
                    },
                    ..Default::default()
                });
            }

            for error in errors.iter() {
                diagnostics.push(Diagnostic {
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: error.message().to_owned(),
                    range: lsp_types::Range {
                        start: offset_to_position(error.span().start(), schema.db.source_assert_single()),
                        end: offset_to_position(error.span().end(), schema.db.source_assert_single()),
                    },
                    ..Default::default()
                });
            }

            Some(diagnostics)
        }
    }
}

pub(crate) fn test_scenario(scenario_name: &str) {
    let mut path = String::with_capacity(SCENARIOS_PATH.len() + 12);

    let schema = {
        write!(path, "{SCENARIOS_PATH}/{scenario_name}/schema.prisma").unwrap();
        std::fs::read_to_string(&path).unwrap()
    };

    let source_file = psl::parser_database::SourceFile::new_allocated(Arc::from(schema.clone().into_boxed_str()));

    let diagnostics = match parse_schema_diagnostics(source_file) {
        Some(diagnostics) => diagnostics,
        None => Vec::new(),
    };

    path.clear();
    write!(path, "{SCENARIOS_PATH}/{scenario_name}/result.json").unwrap();
    let expected_result = std::fs::read_to_string(&path).unwrap_or_else(|_| String::new());

    let params = lsp_types::CodeActionParams {
        text_document: lsp_types::TextDocumentIdentifier {
            uri: "file:/path/to/schema.prisma".parse().unwrap(),
        },
        range: lsp_types::Range::default(),
        context: lsp_types::CodeActionContext {
            diagnostics,
            ..Default::default()
        },
        work_done_progress_params: lsp_types::WorkDoneProgressParams { work_done_token: None },
        partial_result_params: lsp_types::PartialResultParams {
            partial_result_token: None,
        },
    };

    let result = prisma_fmt::code_actions(schema, &serde_json::to_string_pretty(&params).unwrap());
    // Prettify the JSON
    let result =
        serde_json::to_string_pretty(&serde_json::from_str::<Vec<lsp_types::CodeActionOrCommand>>(&result).unwrap())
            .unwrap();

    if *UPDATE_EXPECT {
        let mut file = std::fs::File::create(&path).unwrap(); // truncate
        file.write_all(result.as_bytes()).unwrap();
    } else if expected_result != result {
        let chunks = dissimilar::diff(&expected_result, &result);
        panic!(
            r#"
Snapshot comparison failed. Run the test again with UPDATE_EXPECT=1 in the environment to update the snapshot.

===== EXPECTED ====
{}
====== FOUND ======
{}
======= DIFF ======
{}
"#,
            expected_result,
            result,
            format_chunks(chunks),
        );
    }
}

fn format_chunks(chunks: Vec<dissimilar::Chunk>) -> String {
    let mut buf = String::new();
    for chunk in chunks {
        let formatted = match chunk {
            dissimilar::Chunk::Equal(text) => text.into(),
            dissimilar::Chunk::Delete(text) => format!("\x1b[41m{text}\x1b[0m"),
            dissimilar::Chunk::Insert(text) => format!("\x1b[42m{text}\x1b[0m"),
        };
        buf.push_str(&formatted);
    }
    buf
}
