use enumflags2::BitFlags;
use log::*;
use lsp_types::*;
use psl::{
    datamodel_connector::Connector,
    diagnostics::Span,
    parse_configuration,
    parser_database::{ast, ParserDatabase, SourceFile},
    Configuration, Datasource, Diagnostics, Generator, PreviewFeature,
};
use std::sync::Arc;

use crate::position_to_offset;

mod datasource;

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

    let config = parse_configuration(source_file.as_str()).ok();

    let mut list = CompletionList {
        is_incomplete: false,
        items: Vec::new(),
    };

    let db = {
        let mut diag = Diagnostics::new();
        ParserDatabase::new(source_file, &mut diag)
    };

    let ctx = CompletionContext {
        config: config.as_ref(),
        params: &params,
        db: &db,
        position,
    };

    push_ast_completions(ctx, &mut list);

    list
}

#[derive(Debug, Clone, Copy)]
struct CompletionContext<'a> {
    config: Option<&'a Configuration>,
    params: &'a CompletionParams,
    db: &'a ParserDatabase,
    position: usize,
}

impl<'a> CompletionContext<'a> {
    pub(crate) fn connector(self) -> &'static dyn Connector {
        self.datasource()
            .map(|ds| ds.active_connector)
            .unwrap_or(&psl::datamodel_connector::EmptyDatamodelConnector)
    }

    pub(crate) fn namespaces(self) -> &'a [(String, Span)] {
        self.datasource().map(|ds| ds.namespaces.as_slice()).unwrap_or(&[])
    }

    pub(crate) fn preview_features(self) -> BitFlags<PreviewFeature> {
        self.generator()
            .and_then(|gen| gen.preview_features)
            .unwrap_or_default()
    }

    fn datasource(self) -> Option<&'a Datasource> {
        self.config.and_then(|conf| conf.datasources.first())
    }

    fn generator(self) -> Option<&'a Generator> {
        self.config.and_then(|conf| conf.generators.first())
    }
}

fn push_ast_completions(ctx: CompletionContext<'_>, completion_list: &mut CompletionList) {
    match ctx.db.ast().find_at_position(ctx.position) {
        ast::SchemaPosition::Model(
            _model_id,
            ast::ModelPosition::Field(_, ast::FieldPosition::Attribute("relation", _, Some(attr_name))),
        ) if attr_name == "onDelete" || attr_name == "onUpdate" => {
            for referential_action in ctx.connector().referential_actions().iter() {
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
        ) if ctx.preview_features().contains(PreviewFeature::MultiSchema) => {
            push_namespaces(ctx, completion_list);
        }

        ast::SchemaPosition::Enum(
            _enum_id,
            ast::EnumPosition::EnumAttribute("schema", _, ast::AttributePosition::Attribute),
        ) if ctx.preview_features().contains(PreviewFeature::MultiSchema) => {
            push_namespaces(ctx, completion_list);
        }

        ast::SchemaPosition::DataSource(_source_id, ast::SourcePosition::Source) => {
            datasource::provider_completion(completion_list);
            datasource::url_completion(completion_list);
            datasource::shadow_db_completion(completion_list);
            datasource::direct_url_completion(completion_list);
            datasource::relation_mode_completion(completion_list);

            if ctx
                .connector()
                .has_capability(psl::datamodel_connector::ConnectorCapability::MultiSchema)
                && ctx.namespaces().is_empty()
                && ctx.preview_features().contains(PreviewFeature::MultiSchema)
            {
                datasource::schemas_completion(completion_list);
            }
        }

        position => ctx.connector().push_completions(ctx.db, position, completion_list),
    }
}

fn push_namespaces(ctx: CompletionContext<'_>, completion_list: &mut CompletionList) {
    for (namespace, _) in ctx.namespaces() {
        let insert_text = if add_quotes(ctx.params, ctx.db.source()) {
            format!(r#""{namespace}""#)
        } else {
            namespace.to_string()
        };

        completion_list.items.push(CompletionItem {
            label: String::from(namespace),
            insert_text: Some(insert_text),
            kind: Some(CompletionItemKind::PROPERTY),
            ..Default::default()
        })
    }
}

fn add_quotes(params: &CompletionParams, schema: &str) -> bool {
    if let Some(ctx) = &params.context {
        !(is_inside_quote(&params.text_document_position.position, schema)
            || matches!(ctx.trigger_character.as_deref(), Some("\"")))
    } else {
        false
    }
}

fn is_inside_quote(position: &lsp_types::Position, schema: &str) -> bool {
    match position_to_offset(position, schema) {
        Some(pos) => {
            for i in (0..pos).rev() {
                if schema.is_char_boundary(i) {
                    match schema[(i + 1)..].chars().next() {
                        Some('"') => return true,
                        _ => return false,
                    }
                }
            }
            false
        }
        None => false,
    }
}

fn generate_pretty_doc(example: &str, description: &str) -> String {
    format!("```prisma\n{example}\n```\n___\n{description}")
}
