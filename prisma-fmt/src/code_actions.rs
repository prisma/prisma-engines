mod multi_schema;
mod relations;

use lsp_types::{CodeActionOrCommand, CodeActionParams, Diagnostic, Range, TextEdit, WorkspaceEdit};
use psl::{
    diagnostics::Span,
    parser_database::{
        ast,
        walkers::{EnumWalker, ModelWalker, RefinedRelationWalker, ScalarFieldWalker},
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

    for model in validated_schema.db.walk_models() {
        if validated_schema
            .configuration
            .preview_features()
            .contains(PreviewFeature::MultiSchema)
        {
            multi_schema::add_schema_block_attribute_model(
                &mut actions,
                &params,
                validated_schema.db.source(),
                &validated_schema.configuration,
                model,
            )
        }
    }

    for enumerator in validated_schema.db.walk_enums() {
        if validated_schema
            .configuration
            .preview_features()
            .contains(PreviewFeature::MultiSchema)
        {
            multi_schema::add_schema_block_attribute_enum(
                &mut actions,
                &params,
                validated_schema.db.source(),
                &validated_schema.configuration,
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
        let (new_text, range) = create_block_attribute(schema, model, fields, attribute_name);
        (new_text, range)
    };

    TextEdit { range, new_text }
}

fn create_block_attribute<'a>(
    schema: &str,
    model: ModelWalker<'a>,
    fields: impl ExactSizeIterator<Item = ScalarFieldWalker<'a>> + 'a,
    attribute_name: &str,
) -> (String, Range) {
    let fields = fields.map(|f| f.name()).collect::<Vec<_>>().join(", ");

    let indentation = model.indentation();
    let newline = model.newline();
    let separator = if model.ast_model().attributes.is_empty() {
        newline.as_ref()
    } else {
        ""
    };
    let new_text = format!("{separator}{indentation}@@{attribute_name}([{fields}]){newline}}}");

    let start = crate::offset_to_position(model.ast_model().span().end - 1, schema);
    let end = crate::offset_to_position(model.ast_model().span().end, schema);

    let range = Range { start, end };

    (new_text, range)
}

fn create_schema_attribute_edit_model(schema: &str, model: ModelWalker, params: &CodeActionParams) -> WorkspaceEdit {
    let (new_text, range) = create_schema_attribute(
        schema,
        model.indentation(),
        &model.ast_model().attributes,
        model.newline(),
        model.ast_model().span(),
    );
    let text = TextEdit { range, new_text };

    let mut changes = HashMap::new();
    changes.insert(params.text_document.uri.clone(), vec![text]);

    WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    }
}

fn create_schema_attribute_edit_enum(schema: &str, enumerator: EnumWalker, params: &CodeActionParams) -> WorkspaceEdit {
    let (new_text, range) = create_schema_attribute(
        schema,
        enumerator.indentation(),
        &enumerator.ast_enum().attributes,
        enumerator.newline(),
        enumerator.ast_enum().span(),
    );
    let text = TextEdit { range, new_text };

    let mut changes = HashMap::new();
    changes.insert(params.text_document.uri.clone(), vec![text]);

    WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    }
}

fn create_schema_attribute(
    schema: &str,
    indentation: IndentationType,
    attributes: &Vec<Attribute>,
    newline: NewlineType,
    span: Span,
) -> (String, Range) {
    let seperator = if attributes.is_empty() { newline.as_ref() } else { "" };

    let new_text = format!("{seperator}{indentation}@@schema(\"\"){newline}}}");

    let start = crate::offset_to_position(span.end - 1, schema);
    let end = crate::offset_to_position(span.end, schema);

    let range = Range { start, end };

    (new_text, range)
}
