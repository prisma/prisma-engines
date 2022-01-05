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

    for (line_idx, line) in document.lines().enumerate() {
        if position.line == line_idx as u32 {
            // We're on the right line.
            return if position.character < line.len() as u32 {
                Some(offset + position.character as usize)
            } else {
                None
            };
        }

        // Next line, but first add the current line to the offset.
        offset += line.len() + 1; // don't forget the newline char!
    }

    None
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
