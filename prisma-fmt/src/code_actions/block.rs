use std::collections::HashMap;

use lsp_types::{CodeAction, CodeActionKind, CodeActionOrCommand, Diagnostic, Range, TextEdit, Url, WorkspaceEdit};
use psl::{diagnostics::Span, parser_database::walkers::ModelWalker, schema_ast::ast::WithSpan};

use super::CodeActionsContext;

pub(super) fn create_missing_block(
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
        start: model.ast_model().span().start,
        end: model.ast_model().span().end + 1, // * otherwise it's still not outside the closing brace
        file_id: model.ast_model().span().file_id,
    };

    let range = super::range_after_span(context.initiating_file_source(), span);

    diagnostics.iter().for_each(|diag| {
        push_missing_block(
            diag,
            context.lsp_params.text_document.uri.clone(),
            range,
            "model",
            actions,
        );
        push_missing_block(
            diag,
            context.lsp_params.text_document.uri.clone(),
            range,
            "enum",
            actions,
        );

        if let Some(ds) = context.datasource() {
            if ds.active_provider == "mongodb" {
                push_missing_block(
                    diag,
                    context.lsp_params.text_document.uri.clone(),
                    range,
                    "type",
                    actions,
                );
            }
        }
    })
}

fn push_missing_block(
    diag: &Diagnostic,
    uri: Url,
    range: Range,
    block_type: &str,
    actions: &mut Vec<CodeActionOrCommand>,
) {
    let name: &str = diag.message.split('\"').collect::<Vec<&str>>()[1];
    let new_text = format!("\n{block_type} {name} {{\n\n}}\n");
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
