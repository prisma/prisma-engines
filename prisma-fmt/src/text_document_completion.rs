use datamodel::{
    datamodel_connector::{Connector, Diagnostics, ReferentialIntegrity},
    parse_configuration, parse_schema_ast,
    parser_database::ParserDatabase,
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

    let mut list = CompletionList {
        is_incomplete: false,
        items: Vec::new(),
    };

    let db = {
        let mut diag = Diagnostics::new();
        ParserDatabase::new(schema_ast, &mut diag)
    };

    push_ast_completions(&mut list, connector, referential_integrity, &db, position);

    list
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
    completion_list: &mut CompletionList,
    connector: &'static dyn Connector,
    referential_integrity: ReferentialIntegrity,
    db: &ParserDatabase,
    position: usize,
) {
    match db.ast().find_at_position(position) {
        ast::SchemaPosition::Model(
            _model_id,
            ast::ModelPosition::Field(_, ast::FieldPosition::Attribute("default", _, None)),
        ) => {
            if connector.has_capability(datamodel::datamodel_connector::ConnectorCapability::NamedDefaultValues) {
                completion_list.items.push(CompletionItem {
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
                completion_list.items.push(CompletionItem {
                    label: referential_action.as_str().to_owned(),
                    kind: Some(CompletionItemKind::ENUM),
                    // what is the difference between detail and documentation?
                    detail: Some(referential_action.documentation().to_owned()),
                    ..Default::default()
                });
            }
        }
        ast::SchemaPosition::Model(
            _model_id,
            ast::ModelPosition::Index(_, ast::AttributePosition::Argument("type")),
        ) => {
            for index_type in connector.supported_index_types() {
                completion_list.items.push(CompletionItem {
                    label: index_type.to_string(),
                    kind: Some(CompletionItemKind::ENUM),
                    detail: Some(index_type.documentation().to_owned()),
                    ..Default::default()
                });
            }
        }
        ast::SchemaPosition::Model(
            model_id,
            ast::ModelPosition::Index(attr_id, ast::AttributePosition::FunctionArgument(field_name, "ops")),
        ) => {
            // let's not care about composite field indices yet
            if field_name.contains('.') {
                return;
            }

            let index_field = db
                .walk_models()
                .find(|model| model.model_id() == model_id)
                .and_then(|model| {
                    model.indexes().find(|index| {
                        index.attribute_id()
                            == ast::AttributeId::new_in_container(ast::AttributeContainer::Model(model_id), attr_id)
                    })
                })
                .and_then(|index| {
                    index
                        .fields()
                        .find(|f| f.name() == field_name)
                        .and_then(|f| f.as_scalar_field())
                        .map(|field| (index, field))
                });

            if let Some((index, field)) = index_field {
                let algo = index.algorithm().unwrap_or_default();

                for operator in connector.allowed_index_operator_classes(algo, field) {
                    completion_list.items.push(CompletionItem {
                        label: operator.to_string(),
                        kind: Some(CompletionItemKind::ENUM),
                        ..Default::default()
                    });
                }

                if connector.supports_raw_index_operator_class() {
                    completion_list.items.push(CompletionItem {
                        label: "raw".to_string(),
                        kind: Some(CompletionItemKind::FUNCTION),
                        ..Default::default()
                    });
                }
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
