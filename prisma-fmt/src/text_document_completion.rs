use datamodel::{
    datamodel_connector::{Connector, Diagnostics, ReferentialIntegrity},
    parse_configuration, parse_schema_ast,
    parser_database::ParserDatabase,
    schema_ast::{ast, source_file::SourceFile},
    Datasource,
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
    if parse_schema_ast(&schema).is_err() {
        warn!("Failed to parse schema AST in completion request.");
        return empty_completion_list();
    };
    let source_file = SourceFile::new_allocated(Arc::from(schema.into_boxed_str()));

    let position =
        if let Some(pos) = super::position_to_offset(&params.text_document_position.position, source_file.as_str()) {
            pos
        } else {
            warn!("Received a position outside of the document boundaries in CompletionParams");
            return empty_completion_list();
        };

    let (connector, referential_integrity, datasource) = parse_configuration(source_file.as_str())
        .ok()
        .and_then(|conf| conf.subject.datasources.into_iter().next())
        .map(|datasource| {
            (
                datasource.active_connector,
                datasource.referential_integrity(),
                Some(datasource),
            )
        })
        .unwrap_or_else(|| {
            (
                &datamodel::datamodel_connector::EmptyDatamodelConnector,
                Default::default(),
                None,
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

    push_ast_completions(
        &mut list,
        connector,
        referential_integrity,
        &db,
        position,
        datasource.as_ref(),
    );

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
    datasource: Option<&Datasource>,
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
        ast::SchemaPosition::Model(
            model_id,
            ast::ModelPosition::Field(field_id, ast::FieldPosition::Attribute(attr_name, _, None)),
        ) if datasource
            .map(|ds| attr_name.starts_with(&ds.name) && attr_name.contains('.'))
            .unwrap_or(false) =>
        {
            let field = &db.ast()[model_id][field_id];
            let field_type = if let ast::FieldType::Supported(name) = &field.field_type {
                name.name.as_str()
            } else {
                return;
            };
            let constructors = connector
                .available_native_type_constructors()
                .iter()
                // Only the constructors matching the field's Prisma type.
                .filter(|con| con.prisma_types.iter().any(|pt| pt.as_str() == field_type));
            for constructor in constructors {
                let name = constructor.name.to_owned();
                let args = if constructor.number_of_args + constructor.number_of_optional_args == 0 {
                    ""
                } else {
                    "($0)"
                };

                completion_list.items.push(CompletionItem {
                    label: name.to_owned(),
                    kind: Some(CompletionItemKind::CONSTRUCTOR),
                    insert_text: Some(format!("{name}{args}")),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    ..Default::default()
                });
            }
        }
        position => connector.push_completions(db, position, completion_list),
    }
}
