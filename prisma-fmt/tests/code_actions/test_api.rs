use once_cell::sync::Lazy;
use std::{fmt::Write as _, io::Write as _};

const SCENARIOS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/code_actions/scenarios");
static UPDATE_EXPECT: Lazy<bool> = Lazy::new(|| std::env::var("UPDATE_EXPECT").is_ok());

pub(crate) fn test_scenario(scenario_name: &str) {
    let mut path = String::with_capacity(SCENARIOS_PATH.len() + 12);

    let schema = {
        write!(path, "{}/{}/schema.prisma", SCENARIOS_PATH, scenario_name).unwrap();
        std::fs::read_to_string(&path).unwrap()
    };

    path.clear();
    write!(path, "{}/{}/diagnostics.json", SCENARIOS_PATH, scenario_name).unwrap();
    let diagnostics = std::fs::read_to_string(&path).unwrap_or_else(|_| String::new());
    let diagnostics = serde_json::from_str(&diagnostics).unwrap_or_default();

    path.clear();
    write!(path, "{}/{}/result.json", SCENARIOS_PATH, scenario_name).unwrap();
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
            dissimilar::Chunk::Delete(text) => format!("\x1b[41m{}\x1b[0m", text),
            dissimilar::Chunk::Insert(text) => format!("\x1b[42m{}\x1b[0m", text),
        };
        buf.push_str(&formatted);
    }
    buf
}
