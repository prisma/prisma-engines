use crate::helpers::load_schema_files;
use once_cell::sync::Lazy;
use std::{fmt::Write as _, io::Write as _};

const CURSOR_MARKER: &str = "<|>";
const SCENARIOS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/hover/scenarios");
static UPDATE_EXPECT: Lazy<bool> = Lazy::new(|| std::env::var("UPDATE_EXPECT").is_ok());

pub(crate) fn test_scenario(scenario_name: &str) {
    let mut path = String::with_capacity(SCENARIOS_PATH.len() + 12);

    let schema_files = {
        write!(path, "{SCENARIOS_PATH}/{scenario_name}").unwrap();
        load_schema_files(&path)
    };

    path.clear();
    write!(path, "{SCENARIOS_PATH}/{scenario_name}/result.json").unwrap();
    let expected_result = std::fs::read_to_string(&path).unwrap_or_else(|_| String::new());

    let (initiating_file_uri, cursor_position, schema_files) = take_cursor(schema_files);

    let params = lsp_types::HoverParams {
        text_document_position_params: lsp_types::TextDocumentPositionParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: initiating_file_uri.parse().unwrap(),
            },
            position: cursor_position,
        },
        work_done_progress_params: lsp_types::WorkDoneProgressParams { work_done_token: None },
    };

    let result = prisma_fmt::hover(
        serde_json::to_string_pretty(&schema_files).unwrap(),
        &serde_json::to_string_pretty(&params).unwrap(),
    );

    // Prettify the JSON
    let result = serde_json::to_string_pretty(&serde_json::from_str::<lsp_types::Hover>(&result).unwrap()).unwrap();

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

fn take_cursor(schema_files: Vec<(String, String)>) -> (String, lsp_types::Position, Vec<(String, String)>) {
    let mut result = Vec::with_capacity(schema_files.len());
    let mut file_and_pos = None;
    for (file_name, content) in schema_files {
        if let Some((pos, without_cursor)) = take_cursor_one(&content) {
            file_and_pos = Some((file_name.clone(), pos));
            result.push((file_name, without_cursor));
        } else {
            result.push((file_name, content));
        }
    }

    let (file_name, position) = file_and_pos.expect("Could not find a cursor in any of the schema files");

    (file_name, position, result)
}

fn take_cursor_one(schema: &str) -> Option<(lsp_types::Position, String)> {
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

    if !cursor_found {
        return None;
    }
    // remove extra newline
    schema_without_cursor.truncate(schema_without_cursor.len() - 1);

    Some((cursor_position, schema_without_cursor))
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

    let (pos, schema) = take_cursor_one(schema).unwrap();
    assert_eq!(pos.line, 2);
    assert_eq!(pos.character, 28);
    assert_eq!(schema, expected_schema);
}
