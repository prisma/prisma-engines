use once_cell::sync::Lazy;
use std::{fmt::Write as _, io::Write as _};

const CURSOR_MARKER: &str = "<|>";
const SCENARIOS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/text_document_completion/scenarios");
static UPDATE_EXPECT: Lazy<bool> = Lazy::new(|| std::env::var("UPDATE_EXPECT").is_ok());

pub(crate) fn test_scenario(scenario_name: &str) {
    let mut path = String::with_capacity(SCENARIOS_PATH.len() + 12);

    let schema = {
        write!(path, "{}/{}/schema.prisma", SCENARIOS_PATH, scenario_name).unwrap();
        std::fs::read_to_string(&path).unwrap()
    };

    path.clear();
    write!(path, "{}/{}/result.json", SCENARIOS_PATH, scenario_name).unwrap();
    let expected_result = std::fs::read_to_string(&path).unwrap_or_else(|_| String::new());

    let (cursor_position, schema) = take_cursor(&schema);
    let params = lsp_types::CompletionParams {
        text_document_position: lsp_types::TextDocumentPositionParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: "https://example.com/meow".parse().unwrap(),
            }, // ignored
            position: cursor_position,
        },
        work_done_progress_params: lsp_types::WorkDoneProgressParams { work_done_token: None },
        partial_result_params: lsp_types::PartialResultParams {
            partial_result_token: None,
        },
        context: None,
    };

    let result = prisma_fmt::text_document_completion(&schema, &serde_json::to_string_pretty(&params).unwrap());
    // Prettify the JSON
    let result =
        serde_json::to_string_pretty(&serde_json::from_str::<lsp_types::CompletionList>(&result).unwrap()).unwrap();

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

fn take_cursor(schema: &str) -> (lsp_types::Position, String) {
    let mut schema_without_cursor = String::with_capacity(schema.len() - 3);
    let mut cursor_position = lsp_types::Position { character: 0, line: 0 };
    let mut cursor_found = false;
    for line in schema.lines() {
        if !cursor_found {
            if let Some(pos) = line.find(CURSOR_MARKER) {
                cursor_position.character = pos as u32;
                cursor_found = true;
                schema_without_cursor.push_str(&line[..pos]);
                schema_without_cursor.push_str(&line[pos + 3..]);
                schema_without_cursor.push('\n');
            } else {
                schema_without_cursor.push_str(line);
                schema_without_cursor.push('\n');
                cursor_position.line += 1;
            }
        } else {
            schema_without_cursor.push_str(line);
            schema_without_cursor.push('\n');
        }
    }

    assert!(cursor_found);
    // remove extra newline
    schema_without_cursor.truncate(schema_without_cursor.len() - 1);

    (cursor_position, schema_without_cursor)
}

#[test]
fn take_cursor_works() {
    let schema = r#"
        model Test {
            id Int @id @map(<|>)
        }
    "#;
    let expected_schema = r#"
        model Test {
            id Int @id @map()
        }
    "#;

    let (pos, schema) = take_cursor(schema);
    assert_eq!(pos.line, 2);
    assert_eq!(pos.character, 28);
    assert_eq!(schema, expected_schema);
}
