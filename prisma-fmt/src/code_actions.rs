mod mongodb;
mod multi_schema;
mod relation_mode;
mod relations;

use lsp_types::{CodeActionOrCommand, CodeActionParams, Diagnostic, Range, TextEdit, WorkspaceEdit};
use psl::{
    diagnostics::Span,
    parser_database::{
        ast,
        walkers::{ModelWalker, RefinedRelationWalker, ScalarFieldWalker},
        SourceFile,
    },
    schema_ast::ast::{Attribute, IndentationType, NewlineType, WithSpan},
    PreviewFeature,
};
use std::{collections::HashMap, sync::Arc};

pub(crate) fn empty_code_actions() -> Vec<CodeActionOrCommand> {
    Vec::new()
}

pub(crate) fn available_actions(schema: String, params: CodeActionParams) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();

    let file = SourceFile::new_allocated(Arc::from(schema.into_boxed_str()));

    let validated_schema = psl::validate(file);

    let config = &validated_schema.configuration;

    let datasource = config.datasources.first();

    for source in validated_schema.db.ast_assert_single().sources() {
        relation_mode::edit_referential_integrity(
            &mut actions,
            &params,
            validated_schema.db.source_assert_single(),
            source,
        )
    }

    // models AND views
    for model in validated_schema
        .db
        .walk_models()
        .chain(validated_schema.db.walk_views())
    {
        if config.preview_features().contains(PreviewFeature::MultiSchema) {
            multi_schema::add_schema_block_attribute_model(
                &mut actions,
                &params,
                validated_schema.db.source_assert_single(),
                config,
                model,
            );

            multi_schema::add_schema_to_schemas(
                &mut actions,
                &params,
                validated_schema.db.source_assert_single(),
                config,
                model,
            );
        }

        if matches!(datasource, Some(ds) if ds.active_provider == "mongodb") {
            mongodb::add_at_map_for_id(&mut actions, &params, validated_schema.db.source_assert_single(), model);

            mongodb::add_native_for_auto_id(
                &mut actions,
                &params,
                validated_schema.db.source_assert_single(),
                model,
                datasource.unwrap(),
            );
        }
    }

    for enumerator in validated_schema.db.walk_enums() {
        if config.preview_features().contains(PreviewFeature::MultiSchema) {
            multi_schema::add_schema_block_attribute_enum(
                &mut actions,
                &params,
                validated_schema.db.source_assert_single(),
                config,
                enumerator,
            )
        }
    }

    for relation in validated_schema.db.walk_relations() {
        if let RefinedRelationWalker::Inline(relation) = relation.refine() {
            let complete_relation = match relation.as_complete() {
                Some(relation) => relation,
                None => continue,
            };

            relations::add_referenced_side_unique(
                &mut actions,
                &params,
                validated_schema.db.source_assert_single(),
                complete_relation,
            );

            if relation.is_one_to_one() {
                relations::add_referencing_side_unique(
                    &mut actions,
                    &params,
                    validated_schema.db.source_assert_single(),
                    complete_relation,
                );
            }

            if validated_schema.relation_mode().is_prisma() {
                relations::add_index_for_relation_fields(
                    &mut actions,
                    &params,
                    validated_schema.db.source_assert_single(),
                    complete_relation.referencing_field(),
                );
            }

            if validated_schema.relation_mode().uses_foreign_keys() {
                relation_mode::replace_set_default_mysql(
                    &mut actions,
                    &params,
                    validated_schema.db.source_assert_single(),
                    complete_relation,
                    config,
                )
            }
        }
    }

    actions
}

/// A function to find diagnostics matching the given span. Used for
/// copying the diagnostics to a code action quick fix.
#[track_caller]
pub(super) fn diagnostics_for_span(
    schema: &str,
    diagnostics: &[Diagnostic],
    span: ast::Span,
) -> Option<Vec<Diagnostic>> {
    let res: Vec<_> = diagnostics
        .iter()
        .filter(|diag| span.overlaps(crate::range_to_span(diag.range, schema)))
        .cloned()
        .collect();

    if res.is_empty() {
        None
    } else {
        Some(res)
    }
}

fn filter_diagnostics(span_diagnostics: Vec<Diagnostic>, diagnostic_message: &str) -> Option<Vec<Diagnostic>> {
    let diagnostics = span_diagnostics
        .into_iter()
        .filter(|diag| diag.message.contains(diagnostic_message))
        .collect::<Vec<Diagnostic>>();

    if diagnostics.is_empty() {
        return None;
    }

    Some(diagnostics)
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
        let position = crate::position_after_span(field.ast_field().span(), schema);

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

        let range = range_after_span(schema, model.ast_model().span());
        (formatted_attribute, range)
    };

    TextEdit { range, new_text }
}

fn range_after_span(schema: &str, span: Span) -> Range {
    let start = crate::offset_to_position(span.end - 1, schema);
    let end = crate::offset_to_position(span.end, schema);

    Range { start, end }
}

fn span_to_range(schema: &str, span: Span) -> Range {
    let start = crate::offset_to_position(span.start, schema);
    let end = crate::offset_to_position(span.end, schema);

    Range { start, end }
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
    schema: &str,
    formatted_attribute: String,
    append: bool,
    span: Span,
    params: &CodeActionParams,
) -> WorkspaceEdit {
    let range = match append {
        true => range_after_span(schema, span),
        false => span_to_range(schema, span),
    };

    let text = TextEdit {
        range,
        new_text: formatted_attribute,
    };

    let mut changes = HashMap::new();
    changes.insert(params.text_document.uri.clone(), vec![text]);

    WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    }
}
