mod block;
mod mongodb;
mod multi_schema;
mod relation_mode;
mod relations;

use crate::offsets::{position_after_span, range_to_span, span_to_range};
use log::warn;
use lsp_types::{CodeActionOrCommand, CodeActionParams, Diagnostic, Range, TextEdit, Url, WorkspaceEdit};
use psl::{
    diagnostics::Span,
    parser_database::{
        walkers::{ModelWalker, RefinedRelationWalker, ScalarFieldWalker},
        SourceFile,
    },
    schema_ast::ast::{self, Attribute, IndentationType, NewlineType, WithSpan},
    PreviewFeature,
};
use std::collections::HashMap;

use crate::LSPContext;

pub(super) type CodeActionsContext<'a> = LSPContext<'a, CodeActionParams>;

impl<'a> CodeActionsContext<'a> {
    pub(super) fn diagnostics(&self) -> &[Diagnostic] {
        &self.params.context.diagnostics
    }

    /// A function to find diagnostics matching the given span. Used for
    /// copying the diagnostics to a code action quick fix.
    #[track_caller]
    pub(super) fn diagnostics_for_span(&self, span: ast::Span) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics().iter().filter(move |diag| {
            span.overlaps(range_to_span(
                diag.range,
                self.initiating_file_source(),
                self.initiating_file_id,
            ))
        })
    }
    pub(super) fn diagnostics_for_span_with_message(&self, span: Span, message: &str) -> Vec<Diagnostic> {
        self.diagnostics_for_span(span)
            .filter(|diag| diag.message.contains(message))
            .cloned()
            .collect()
    }
}

pub(crate) fn empty_code_actions() -> Vec<CodeActionOrCommand> {
    Vec::new()
}

pub(crate) fn available_actions(
    schema_files: Vec<(String, SourceFile)>,
    params: CodeActionParams,
) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();

    let validated_schema = psl::validate_multi_file(&schema_files);

    let config = &validated_schema.configuration;

    let datasource = config.datasources.first();
    let file_uri = params.text_document.uri.as_str();
    let Some(initiating_file_id) = validated_schema.db.file_id(file_uri) else {
        warn!("Initiating file name is not found in the schema");
        return vec![];
    };

    let context = CodeActionsContext {
        db: &validated_schema.db,
        config,
        initiating_file_id,
        params: &params,
    };

    let initiating_ast = validated_schema.db.ast(initiating_file_id);
    for source in initiating_ast.sources() {
        relation_mode::edit_referential_integrity(&mut actions, &context, source)
    }

    // models AND views
    for model in validated_schema
        .db
        .walk_models_in_file(initiating_file_id)
        .chain(validated_schema.db.walk_views_in_file(initiating_file_id))
    {
        block::create_missing_block_for_model(&mut actions, &context, model);

        if config.preview_features().contains(PreviewFeature::MultiSchema) {
            multi_schema::add_schema_block_attribute_model(&mut actions, &context, model);

            multi_schema::add_schema_to_schemas(&mut actions, &context, model);
        }

        if matches!(datasource, Some(ds) if ds.active_provider == "mongodb") {
            mongodb::add_at_map_for_id(&mut actions, &context, model);

            mongodb::add_native_for_auto_id(&mut actions, &context, model, datasource.unwrap());
        }
    }

    if matches!(datasource, Some(ds) if ds.active_provider == "mongodb") {
        for composite_type in validated_schema.db.walk_composite_types_in_file(initiating_file_id) {
            block::create_missing_block_for_type(&mut actions, &context, composite_type);
        }
    }

    for enumerator in validated_schema.db.walk_enums_in_file(initiating_file_id) {
        if config.preview_features().contains(PreviewFeature::MultiSchema) {
            multi_schema::add_schema_block_attribute_enum(&mut actions, &context, enumerator);
        }
    }

    for relation in validated_schema.db.walk_relations() {
        if let RefinedRelationWalker::Inline(relation) = relation.refine() {
            let complete_relation = match relation.as_complete() {
                Some(relation) => relation,
                None => continue,
            };

            relations::add_referenced_side_unique(&mut actions, &context, complete_relation);

            if relation.is_one_to_one() {
                relations::add_referencing_side_unique(&mut actions, &context, complete_relation);
            }

            if validated_schema.relation_mode().is_prisma()
                && relation.referencing_model().is_defined_in_file(initiating_file_id)
            {
                relations::add_index_for_relation_fields(&mut actions, &context, complete_relation.referencing_field());
            }

            if validated_schema.relation_mode().uses_foreign_keys() {
                relation_mode::replace_set_default_mysql(&mut actions, &context, complete_relation)
            }
        }
    }

    actions
}

fn create_missing_attribute<'a>(
    schema: &str,
    model: ModelWalker<'a>,
    mut fields: impl ExactSizeIterator<Item = ScalarFieldWalker<'a>> + 'a,
    attribute_name: &str,
) -> TextEdit {
    let (new_text, range) = if fields.len() == 1 {
        let new_text = format!(" @{attribute_name}");

        let field = fields.next().unwrap();
        let position = position_after_span(field.ast_field().span(), schema);

        let range = Range {
            start: position,
            end: position,
        };

        (new_text, range)
    } else {
        let fields = fields.map(|f| f.name()).collect::<Vec<_>>().join(", ");

        let attribute = format!("{attribute_name}([{fields}])");

        let formatted_attribute = format_block_attribute(
            &attribute,
            model.indentation(),
            model.newline(),
            &model.ast_model().attributes,
        );

        let range = range_after_span(model.ast_model().span(), schema);
        (formatted_attribute, range)
    };

    TextEdit { range, new_text }
}

fn range_after_span(span: Span, schema: &str) -> Range {
    span_to_range(Span::new(span.end - 1, span.end, span.file_id), schema)
}

fn format_field_attribute(attribute: &str) -> String {
    // ? (soph) rust doesn't recognise \s
    format!(" {attribute}\n")
}

fn format_block_property(
    property: &str,
    value: &str,
    indentation: IndentationType,
    newline: NewlineType,
    has_properties: bool,
) -> String {
    let separator = if has_properties { newline.as_ref() } else { "" };

    // * (soph) I don't super like needing to prefix this with ')' but
    // * it would require further updating how we parse spans
    // todo: update so that we have a concepts for:
    // todo: - The entire url span
    // todo: - The url arg span :: currently, url_span only represents this.
    let formatted_attribute = format!(r#"){separator}{indentation}{property} = ["{value}"]"#);

    formatted_attribute
}

fn format_block_attribute(
    attribute: &str,
    indentation: IndentationType,
    newline: NewlineType,
    attributes: &[Attribute],
) -> String {
    let separator = if attributes.is_empty() { newline.as_ref() } else { "" };

    let formatted_attribute = format!("{separator}{indentation}@@{attribute}{newline}}}");

    formatted_attribute
}

fn create_text_edit(
    target_file_uri: &str,
    target_file_content: &str,
    formatted_attribute: String,
    append: bool,
    span: Span,
) -> Result<WorkspaceEdit, Box<dyn std::error::Error>> {
    let range = match append {
        true => range_after_span(span, target_file_content),
        false => span_to_range(span, target_file_content),
    };

    let text = TextEdit {
        range,
        new_text: formatted_attribute,
    };

    let mut changes = HashMap::new();
    let url = parse_url(target_file_uri)?;
    changes.insert(url, vec![text]);

    Ok(WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    })
}

pub(crate) fn parse_url(url: &str) -> Result<Url, Box<dyn std::error::Error>> {
    let result = Url::parse(url);
    if result.is_err() {
        warn!("Could not parse url {url}")
    }
    Ok(result?)
}
