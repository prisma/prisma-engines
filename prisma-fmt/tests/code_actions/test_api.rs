use lsp_types::{Diagnostic, DiagnosticSeverity};
use once_cell::sync::Lazy;
use prisma_fmt::span_to_range;
use psl::{diagnostics::Span, SourceFile};
use std::{fmt::Write as _, io::Write as _, path::PathBuf};

use crate::helpers::load_schema_files;

const SCENARIOS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/code_actions/scenarios");
/**
 * Code actions are requested only for single file. So, when emulating lsp request
 * we need a way to designate that file somehow.
 */
const TARGET_SCHEMA_FILE: &str = "_target.prisma";
static UPDATE_EXPECT: Lazy<bool> = Lazy::new(|| std::env::var("UPDATE_EXPECT").is_ok());

fn parse_schema_diagnostics(files: &[(String, String)], initiating_file_name: &str) -> Option<Vec<Diagnostic>> {
    let sources: Vec<_> = files
        .iter()
        .map(|(name, content)| (name.to_owned(), SourceFile::from(content)))
        .collect();
    let schema = psl::validate_multi_file(&sources);

    let file_id = schema.db.file_id(initiating_file_name).unwrap();
    let source = schema.db.source(file_id);
    match (schema.diagnostics.warnings(), schema.diagnostics.errors()) {
        ([], []) => None,
        (warnings, errors) => {
            let mut diagnostics = Vec::new();
            for warn in warnings.iter() {
                if warn.span().file_id == file_id {
                    diagnostics.push(create_diagnostic(
                        DiagnosticSeverity::WARNING,
                        warn.message(),
                        warn.span(),
                        source,
                    ));
                }
            }

            for error in errors.iter() {
                if error.span().file_id == file_id {
                    diagnostics.push(create_diagnostic(
                        DiagnosticSeverity::ERROR,
                        error.message(),
                        error.span(),
                        source,
                    ));
                }
            }

            Some(diagnostics)
        }
    }
}

fn create_diagnostic(severity: DiagnosticSeverity, message: &str, span: Span, source: &str) -> Diagnostic {
    Diagnostic {
        severity: Some(severity),
        message: message.to_owned(),
        range: span_to_range(span, source),
        ..Default::default()
    }
}

pub(crate) fn test_scenario(scenario_name: &str) {
    let mut path = String::with_capacity(SCENARIOS_PATH.len() + 12);

    let schema_files = {
        write!(path, "{SCENARIOS_PATH}/{scenario_name}").unwrap();
        load_schema_files(&path)
    };

    let initiating_file_name = if schema_files.len() == 1 {
        schema_files[0].0.as_str()
    } else {
        schema_files
            .iter()
            .find_map(|(file_path, _)| {
                let path = PathBuf::from(file_path);
                let file_name = path.file_name()?;
                if file_name == TARGET_SCHEMA_FILE {
                    Some(file_path)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| panic!("Expected to have {TARGET_SCHEMA_FILE} in when multi-file schema are used"))
            .as_str()
    };

    let diagnostics = match parse_schema_diagnostics(&schema_files, initiating_file_name) {
        Some(diagnostics) => diagnostics,
        None => Vec::new(),
    };

    path.clear();
    write!(path, "{SCENARIOS_PATH}/{scenario_name}/result.json").unwrap();
    let expected_result = std::fs::read_to_string(&path).unwrap_or_else(|_| String::new());

    let params = lsp_types::CodeActionParams {
        text_document: lsp_types::TextDocumentIdentifier {
            uri: initiating_file_name.parse().unwrap(),
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

    let result = prisma_fmt::code_actions(
        serde_json::to_string_pretty(&schema_files).unwrap(),
        &serde_json::to_string_pretty(&params).unwrap(),
    );
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
