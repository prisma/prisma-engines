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
mod multi_schema;
mod referential_actions;

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
        ParserDatabase::new_single_file(source_file, &mut diag)
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
    match ctx.db.ast_assert_single().find_at_position(ctx.position) {
        ast::SchemaPosition::Model(
            _model_id,
            ast::ModelPosition::Field(_, ast::FieldPosition::Attribute("relation", _, Some(attr_name))),
        ) if attr_name == "onDelete" || attr_name == "onUpdate" => {
            for referential_action in ctx.connector().referential_actions().iter() {
                referential_actions::referential_action_completion(completion_list, referential_action)
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
            if !ds_has_prop(ctx, "provider") {
                datasource::provider_completion(completion_list);
            }

            if !ds_has_prop(ctx, "url") {
                datasource::url_completion(completion_list);
            }

            if !ds_has_prop(ctx, "shadowDatabaseUrl") {
                datasource::shadow_db_completion(completion_list);
            }

            if !ds_has_prop(ctx, "directUrl") {
                datasource::direct_url_completion(completion_list);
            }

            if !ds_has_prop(ctx, "relationMode") {
                datasource::relation_mode_completion(completion_list);
            }

            if let Some(config) = ctx.config {
                ctx.connector().datasource_completions(config, completion_list);
            }
        }

        ast::SchemaPosition::DataSource(
            _source_id,
            ast::SourcePosition::Property("url", ast::PropertyPosition::FunctionValue("env")),
        ) => datasource::url_env_db_completion(completion_list, "url", ctx),

        ast::SchemaPosition::DataSource(
            _source_id,
            ast::SourcePosition::Property("directUrl", ast::PropertyPosition::FunctionValue("env")),
        ) => datasource::url_env_db_completion(completion_list, "directUrl", ctx),

        ast::SchemaPosition::DataSource(
            _source_id,
            ast::SourcePosition::Property("shadowDatabaseUrl", ast::PropertyPosition::FunctionValue("env")),
        ) => datasource::url_env_db_completion(completion_list, "shadowDatabaseUrl", ctx),

        ast::SchemaPosition::DataSource(_source_id, ast::SourcePosition::Property("url", _))
        | ast::SchemaPosition::DataSource(_source_id, ast::SourcePosition::Property("directUrl", _))
        | ast::SchemaPosition::DataSource(_source_id, ast::SourcePosition::Property("shadowDatabaseUrl", _)) => {
            datasource::url_env_completion(completion_list);
            datasource::url_quotes_completion(completion_list);
        }

        position => ctx.connector().datamodel_completions(ctx.db, position, completion_list),
    }
}

fn ds_has_prop(ctx: CompletionContext<'_>, prop: &str) -> bool {
    if let Some(ds) = ctx.datasource() {
        match prop {
            "relationMode" => ds.relation_mode_defined(),
            "directurl" => ds.direct_url_defined(),
            "shadowDatabaseUrl" => ds.shadow_url_defined(),
            "url" => ds.url_defined(),
            "provider" => ds.provider_defined(),
            _ => false,
        }
    } else {
        false
    }
}

fn push_namespaces(ctx: CompletionContext<'_>, completion_list: &mut CompletionList) {
    for (namespace, _) in ctx.namespaces() {
        let insert_text = if add_quotes(ctx.params, ctx.db.source_assert_single()) {
            format!(r#""{namespace}""#)
        } else {
            namespace.to_string()
        };

        multi_schema::schema_namespace_completion(completion_list, namespace, insert_text);
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
