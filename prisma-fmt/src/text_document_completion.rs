use log::*;
use lsp_types::*;
use psl::{
    datamodel_connector::{Connector, RelationMode},
    diagnostics::Span,
    parse_configuration,
    parser_database::{ast, ParserDatabase, SourceFile},
    Diagnostics, PreviewFeature, PreviewFeatures,
};
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

    let configs = parse_configuration(source_file.as_str()).ok();

    let (connector, relation_mode, schemas_span) = configs
        .as_ref()
        .and_then(|conf| conf.datasources.first())
        .map(|datasource| {
            (
                datasource.active_connector,
                datasource.relation_mode(),
                datasource.schemas_span,
            )
        })
        .unwrap_or_else(|| {
            (
                &psl::datamodel_connector::EmptyDatamodelConnector,
                Default::default(),
                Default::default(),
            )
        });

    let preview_features: PreviewFeatures = configs
        .as_ref()
        .and_then(|config| config.generators.first())
        .and_then(|generator| generator.preview_features)
        .unwrap_or_default();

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
        relation_mode,
        &db,
        position,
        schemas_span,
        preview_features,
    );

    list
}

// Completion is implemented for:
// - referential actions (onDelete and onUpdate arguments)
// - default arguments on scalar fields (based on connector capabilities for the `map: ...` argument).
fn push_ast_completions(
    completion_list: &mut CompletionList,
    connector: &'static dyn Connector,
    _relation_mode: RelationMode,
    db: &ParserDatabase,
    position: usize,
    schemas_span: Option<Span>,
    preview_features: PreviewFeatures,
) {
    match db.ast().find_at_position(position) {
        ast::SchemaPosition::Model(
            _model_id,
            ast::ModelPosition::Field(_, ast::FieldPosition::Attribute("relation", _, Some(attr_name))),
        ) if attr_name == "onDelete" || attr_name == "onUpdate" => {
            for referential_action in connector.referential_actions().iter() {
                completion_list.items.push(CompletionItem {
                    label: referential_action.as_str().to_owned(),
                    kind: Some(CompletionItemKind::ENUM),
                    detail: Some(referential_action.documentation().to_owned()),
                    ..Default::default()
                });
            }
        }

        ast::SchemaPosition::DataSource(_source_id, ast::SourcePosition::Source) => {
            if connector.has_capability(psl::datamodel_connector::ConnectorCapability::MultiSchema)
                && schemas_span.is_none()
                && preview_features.contains(PreviewFeature::MultiSchema)
            {
                completion_list.items.push(CompletionItem {
                    label: "schemas".to_owned(),
                    insert_text: Some("schemas = []".to_owned()),
                    kind: Some(CompletionItemKind::PROPERTY),
                    documentation: Some(Documentation::String("The list of database schemas.".to_owned())),
                    ..Default::default()
                });
            }
        }

        position => connector.push_completions(db, position, completion_list),
    }
}
