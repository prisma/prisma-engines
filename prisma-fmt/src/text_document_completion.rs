use datamodel::{
    datamodel_connector::{Connector, ReferentialIntegrity},
    parse_configuration, parse_schema_ast,
    schema_ast::ast,
};
use log::*;
use lsp_types::*;

pub(crate) fn empty_completion_list() -> CompletionList {
    CompletionList {
        is_incomplete: true,
        items: Vec::new(),
    }
}

pub(crate) fn completion(schema: &str, params: CompletionParams) -> CompletionList {
    let schema_ast = if let Ok(schema_ast) = parse_schema_ast(schema) {
        schema_ast
    } else {
        warn!("Failed to parse schema AST in completion request.");
        return empty_completion_list();
    };

    let position = if let Some(pos) = position_to_offset(&params.text_document_position.position, schema) {
        pos
    } else {
        warn!("Received a position outside of the document boundaries in CompletionParams");
        return empty_completion_list();
    };

    let (connector, referential_integrity) = parse_configuration(schema)
        .ok()
        .and_then(|conf| conf.subject.datasources.into_iter().next())
        .map(|datasource| (datasource.active_connector, datasource.referential_integrity()))
        .unwrap_or_else(|| {
            (
                &datamodel::datamodel_connector::EmptyDatamodelConnector,
                Default::default(),
            )
        });

    let mut items = Vec::new();

    push_ast_completions(&mut items, connector, referential_integrity, &schema_ast, position);

    CompletionList {
        is_incomplete: items.is_empty(),
        items,
    }
}

/// The LSP position is expressed as a (line, col) tuple, but our pest-based parser works with byte
/// offsets. This function converts from an LSP position to a pest byte offset. Returns `None` if
/// the position has a line past the end of the document, or a character position past the end of
/// the line.
fn position_to_offset(position: &Position, document: &str) -> Option<usize> {
    let mut offset = 0;
    let mut line_offset = position.line;
    let mut character_offset = position.character;
    let mut chars = document.chars();

    while line_offset > 0 {
        loop {
            match chars.next() {
                Some('\n') => {
                    offset += 1;
                    break;
                }
                Some(_) => {
                    offset += 1;
                }
                None => return None,
            }
        }

        line_offset -= 1;
    }

    while character_offset > 0 {
        match chars.next() {
            Some('\n') | None => return None,
            Some(_) => {
                offset += 1;
                character_offset -= 1;
            }
        }
    }

    Some(offset)
}

// Completion is implemented for:
// - referential actions (onDelete and onUpdate arguments)
// - default arguments on scalar fields (based on connector capabilities for the `map: ...` argument).
fn push_ast_completions(
    items: &mut Vec<CompletionItem>,
    connector: &'static dyn Connector,
    referential_integrity: ReferentialIntegrity,
    ast: &ast::SchemaAst,
    position: usize,
) {
    match ast.find_at_position(position) {
        ast::SchemaPosition::Model(
            _model_id,
            ast::ModelPosition::Field(_, ast::FieldPosition::Attribute("default", _, None)),
        ) => {
            if connector.has_capability(datamodel::datamodel_connector::ConnectorCapability::NamedDefaultValues) {
                items.push(CompletionItem {
                    label: "map: ".to_owned(),
                    kind: Some(CompletionItemKind::PROPERTY),
                    ..Default::default()
                })
            }
        }
        ast::SchemaPosition::Model(
            _model_id,
            ast::ModelPosition::Field(_, ast::FieldPosition::Attribute("relation", _, Some(attr_name))),
        ) if attr_name == "onDelete" || attr_name == "onUpdate" => {
            for referential_action in connector.referential_actions(&referential_integrity).iter() {
                items.push(CompletionItem {
                    label: referential_action.as_str().to_owned(),
                    kind: Some(CompletionItemKind::ENUM),
                    detail: None, // what is the difference between detail and documentation?
                    documentation: Some(Documentation::String(referential_action.documentation().to_owned())),
                    ..Default::default()
                });
            }
        }
        _ => (),
    }
}

// On Windows, a newline is actually two characters.
#[test]
fn position_to_offset_with_crlf() {
    let schema = "\r\nmodel Test {\r\n    id Int @id\r\n}";
    // Let's put the cursor on the "i" in "id Int".
    let expected_offset = schema.chars().position(|c| c == 'i').unwrap();
    let found_offset = position_to_offset(&Position { line: 2, character: 4 }, schema).unwrap();

    assert_eq!(found_offset, expected_offset);
}
