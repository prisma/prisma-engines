use crate::offsets::position_to_offset;
use enumflags2::BitFlags;
use log::*;
use lsp_types::*;
use psl::{
    Diagnostics, PreviewFeature,
    diagnostics::Span,
    error_tolerant_parse_configuration,
    parser_database::{ParserDatabase, SourceFile, ast},
    schema_ast::ast::AttributePosition,
};

use crate::LSPContext;

mod datasource;
mod multi_schema;
mod referential_actions;

pub(super) type CompletionContext<'a> = LSPContext<'a, CompletionParams>;

impl<'a> CompletionContext<'a> {
    pub(super) fn namespaces(&'a self) -> &'a [(String, Span)] {
        self.datasource().map(|ds| ds.namespaces.as_slice()).unwrap_or(&[])
    }

    #[allow(dead_code)]
    pub(super) fn preview_features(&self) -> BitFlags<PreviewFeature> {
        self.generator()
            .and_then(|generator| generator.preview_features)
            .unwrap_or_default()
    }

    pub(super) fn position(&self) -> Option<usize> {
        let pos = self.params.text_document_position.position;
        let initiating_doc = self.initiating_file_source();

        position_to_offset(&pos, initiating_doc)
    }
}

pub(crate) fn empty_completion_list() -> CompletionList {
    CompletionList {
        is_incomplete: true,
        items: Vec::new(),
    }
}

pub(crate) fn completion(schema_files: Vec<(String, SourceFile)>, params: CompletionParams) -> CompletionList {
    let (_, config, _) = error_tolerant_parse_configuration(&schema_files);

    let mut list = CompletionList {
        is_incomplete: false,
        items: Vec::new(),
    };

    let db = {
        let mut diag = Diagnostics::new();
        ParserDatabase::new(&schema_files, &mut diag)
    };

    let Some(initiating_file_id) = db.file_id(params.text_document_position.text_document.uri.as_str()) else {
        warn!("Initiating file name is not found in the schema");
        return empty_completion_list();
    };

    let ctx = CompletionContext {
        config: &config,
        params: &params,
        db: &db,
        initiating_file_id,
    };

    push_ast_completions(ctx, &mut list);

    list
}

fn push_ast_completions(ctx: CompletionContext<'_>, completion_list: &mut CompletionList) {
    let position = match ctx.position() {
        Some(pos) => pos,
        None => {
            warn!("Received a position outside of the document boundaries in CompletionParams");
            completion_list.is_incomplete = true;
            return;
        }
    };

    let relation_mode = ctx
        .config
        .relation_mode()
        .unwrap_or_else(|| ctx.connector().default_relation_mode());

    let find_at_position = ctx.db.ast(ctx.initiating_file_id).find_at_position(position);

    match find_at_position {
        ast::SchemaPosition::Model(
            _model_id,
            ast::ModelPosition::Field(
                _,
                ast::FieldPosition::Attribute("relation", _, AttributePosition::Argument(attr_name)),
            ),
        ) if attr_name == "onDelete" || attr_name == "onUpdate" => {
            for referential_action in ctx.connector().referential_actions(&relation_mode).iter() {
                referential_actions::referential_action_completion(completion_list, referential_action);
            }
        }

        ast::SchemaPosition::Model(
            _model_id,
            ast::ModelPosition::Field(
                _,
                ast::FieldPosition::Attribute("relation", _, AttributePosition::ArgumentValue(attr_name, value)),
            ),
        ) => {
            if let Some(attr_name) = attr_name {
                if attr_name == "onDelete" || attr_name == "onUpdate" {
                    ctx.connector()
                        .referential_actions(&relation_mode)
                        .iter()
                        .filter(|ref_action| ref_action.to_string().starts_with(&value))
                        .for_each(|referential_action| {
                            referential_actions::referential_action_completion(completion_list, referential_action)
                        });
                }
            }
        }

        ast::SchemaPosition::Model(
            _model_id,
            ast::ModelPosition::ModelAttribute("schema", _, ast::AttributePosition::Attribute),
        ) => {
            push_namespaces(ctx, completion_list);
        }

        ast::SchemaPosition::Enum(
            _enum_id,
            ast::EnumPosition::EnumAttribute("schema", _, ast::AttributePosition::Attribute),
        ) => {
            push_namespaces(ctx, completion_list);
        }

        ast::SchemaPosition::DataSource(_source_id, ast::SourcePosition::Source) => {
            if !ds_has_prop(&ctx, "provider") {
                datasource::provider_completion(completion_list);
            }

            if !ds_has_prop(&ctx, "url") {
                datasource::url_completion(completion_list);
            }

            if !ds_has_prop(&ctx, "shadowDatabaseUrl") {
                datasource::shadow_db_completion(completion_list);
            }

            if !ds_has_prop(&ctx, "directUrl") {
                datasource::direct_url_completion(completion_list);
            }

            if !ds_has_prop(&ctx, "relationMode") {
                datasource::relation_mode_completion(completion_list);
            }

            ctx.connector().datasource_completions(ctx.config, completion_list);
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

fn ds_has_prop(ctx: &CompletionContext<'_>, prop: &str) -> bool {
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
        let insert_text = if add_quotes(ctx.params, ctx.db.source(ctx.initiating_file_id)) {
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
