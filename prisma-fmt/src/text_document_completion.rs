use datamodel::{
    datamodel_connector::{Connector, Diagnostics, ReferentialIntegrity},
    parse_configuration,
    parser_database::{ast, ParserDatabase, SourceFile},
};
use log::*;
use lsp_types::*;
use std::sync::Arc;

pub(crate) fn empty_completion_list() -> CompletionList {
    CompletionList {
        is_incomplete: true,
        items: Vec::new(),
    }
}

pub(crate) fn completion(schema: String, params: CompletionParams) -> CompletionList {
    let source_file = SourceFile::new_allocated(Arc::from(schema.into_boxed_str()));

    let position =
        if let Some(pos) = super::position_to_offset(&params.text_document_position.position, source_file.as_str()) {
            pos
        } else {
            warn!("Received a position outside of the document boundaries in CompletionParams");
            return empty_completion_list();
        };

    let (connector, referential_integrity) = parse_configuration(source_file.as_str())
        .ok()
        .and_then(|conf| conf.datasources.into_iter().next())
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
        ParserDatabase::new(source_file, &mut diag)
    };

    push_ast_completions(&mut list, connector, referential_integrity, &db, position);

    list
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
        position => connector.push_completions(db, position, completion_list),
    }
}
