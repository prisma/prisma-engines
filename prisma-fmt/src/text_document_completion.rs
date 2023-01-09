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

    let (connector, relation_mode, namespaces) = configs
        .as_ref()
        .and_then(|conf| conf.datasources.first())
        .map(|datasource| {
            (
                datasource.active_connector,
                datasource.relation_mode(),
                datasource.namespaces.clone(),
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

    let add_quotes = add_quotes(params);

    push_ast_completions(
        &mut list,
        connector,
        relation_mode,
        &db,
        position,
        preview_features,
        namespaces,
        add_quotes,
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
    preview_features: PreviewFeatures,
    namespaces: Vec<(String, Span)>,
    add_quotes: bool,
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
                    // what is the difference between detail and documentation?
                    detail: Some(referential_action.documentation().to_owned()),
                    ..Default::default()
                });
            }
        }

        ast::SchemaPosition::Model(
            _model_id,
            ast::ModelPosition::ModelAttribute("schema", _, ast::AttributePosition::Attribute),
        ) => {
            if connector.has_capability(psl::datamodel_connector::ConnectorCapability::MultiSchema)
                && preview_features.contains(PreviewFeature::MultiSchema)
            {
                for (namespace, _) in namespaces {
                    completion_list.items.push(CompletionItem {
                        label: String::from(&namespace),
                        insert_text: Some(format_insert_string(add_quotes, &namespace)),
                        kind: Some(CompletionItemKind::PROPERTY),
                        documentation: Some(Documentation::String("documentation".to_owned())),
                        ..Default::default()
                    })
                }
            }
        }

        position => connector.push_completions(db, position, completion_list),
    }
}

fn format_insert_string(add_quotes: bool, text: &str) -> String {
    match add_quotes {
        true => format!(r#""{}""#, &text),
        false => text.to_string(),
    }
}

fn add_quotes(params: CompletionParams) -> bool {
    match params.context {
        Some(ctx) => {
            !(ctx.trigger_kind == CompletionTriggerKind::TRIGGER_CHARACTER
                && matches!(ctx.trigger_character, Some(c) if c == "\""))
            // || (params.text_document_position.position.character - 1 == "\""
            //     && params.text_document_position.position.character + 1 == "\"")
        }
        None => true,
    }
}
