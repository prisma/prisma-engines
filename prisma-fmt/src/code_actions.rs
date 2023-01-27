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

    for source in validated_schema.db.ast().sources() {
        relation_mode::edit_referential_integrity(&mut actions, &params, validated_schema.db.source(), source)
    }

    for model in validated_schema
        .db
        .walk_models()
        .chain(validated_schema.db.walk_views())
    {
        if config.preview_features().contains(PreviewFeature::MultiSchema) {
            multi_schema::add_schema_block_attribute_model(
                &mut actions,
                &params,
                validated_schema.db.source(),
                config,
                model,
            )
        }
    }

    for enumerator in validated_schema.db.walk_enums() {
        if config.preview_features().contains(PreviewFeature::MultiSchema) {
            multi_schema::add_schema_block_attribute_enum(
                &mut actions,
                &params,
                validated_schema.db.source(),
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
                validated_schema.db.source(),
                complete_relation,
            );

            if relation.is_one_to_one() {
                relations::add_referencing_side_unique(
                    &mut actions,
                    &params,
                    validated_schema.db.source(),
                    complete_relation,
                );
            }

            if validated_schema.relation_mode().is_prisma() {
                relations::add_index_for_relation_fields(
                    &mut actions,
                    &params,
                    validated_schema.db.source(),
                    complete_relation.referencing_field(),
                );
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

        let formatted_attribute = format_attribute(
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

fn format_attribute(
    attribute: &str,
    indentation: IndentationType,
    newline: NewlineType,
    attributes: &Vec<Attribute>,
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
