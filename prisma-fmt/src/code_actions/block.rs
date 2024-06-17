use std::collections::HashMap;

use lsp_types::{CodeAction, CodeActionKind, CodeActionOrCommand, Diagnostic, Range, TextEdit, Url, WorkspaceEdit};
use psl::{
    diagnostics::Span,
    parser_database::walkers::{CompositeTypeWalker, ModelWalker},
    schema_ast::ast::{NewlineType, WithSpan},
};

use super::CodeActionsContext;

pub(super) fn create_missing_block_for_model(
    actions: &mut Vec<CodeActionOrCommand>,
    context: &CodeActionsContext<'_>,
    model: ModelWalker<'_>,
) {
    let span_model = model.ast_model().span();
    let diagnostics = context
        .diagnostics_for_span_with_message(span_model, "is neither a built-in type, nor refers to another model,");

    if diagnostics.is_empty() {
        return;
    }

    let span = Span {
        start: span_model.start,
        end: span_model.end + 1, // * otherwise it's still not outside the closing brace
        file_id: span_model.file_id,
    };

    let range = super::range_after_span(span, context.initiating_file_source());

    diagnostics.iter().for_each(|diag| {
        push_missing_block(
            diag,
            context.params.text_document.uri.clone(),
            range,
            "model",
            actions,
            model.newline(),
        );
        push_missing_block(
            diag,
            context.params.text_document.uri.clone(),
            range,
            "enum",
            actions,
            model.newline(),
        );

        if let Some(ds) = context.datasource() {
            if ds.active_provider == "mongodb" {
                push_missing_block(
                    diag,
                    context.params.text_document.uri.clone(),
                    range,
                    "type",
                    actions,
                    model.newline(),
                );
            }
        }
    })
}

pub(super) fn create_missing_block_for_type(
    actions: &mut Vec<CodeActionOrCommand>,
    context: &CodeActionsContext<'_>,
    composite_type: CompositeTypeWalker<'_>,
) {
    let span_type = composite_type.ast_composite_type().span;

    let diagnostics = context
        .diagnostics_for_span_with_message(span_type, "is neither a built-in type, nor refers to another model,");

    if diagnostics.is_empty() {
        return;
    }

    let span = Span {
        start: span_type.start,
        end: span_type.end + 1, // * otherwise it's still not outside the closing brace
        file_id: span_type.file_id,
    };

    let range = super::range_after_span(span, context.initiating_file_source());
    diagnostics.iter().for_each(|diag| {
        push_missing_block(
            diag,
            context.params.text_document.uri.clone(),
            range,
            "type",
            actions,
            composite_type.newline(),
        );
        push_missing_block(
            diag,
            context.params.text_document.uri.clone(),
            range,
            "enum",
            actions,
            composite_type.newline(),
        );
    })
}

fn push_missing_block(
    diag: &Diagnostic,
    uri: Url,
    range: Range,
    block_type: &str,
    actions: &mut Vec<CodeActionOrCommand>,
    newline: NewlineType,
) {
    let name: &str = diag.message.split('\"').collect::<Vec<&str>>()[1];
    let new_text = format!("{newline}{block_type} {name} {{{newline}{newline}}}{newline}");
    let text = TextEdit { range, new_text };

    let mut changes = HashMap::new();
    changes.insert(uri, vec![text]);

    let edit = WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    };

    let action = CodeAction {
        title: format!("Create new {block_type} '{name}'"),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics: Some(vec![diag.clone()]),
        ..Default::default()
    };

    actions.push(CodeActionOrCommand::CodeAction(action))
}
