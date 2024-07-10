use lsp_types::{CodeAction, CodeActionKind, CodeActionOrCommand};
use psl::{
    diagnostics::Span,
    parser_database::walkers::{self, FieldWalker},
    schema_ast::ast::{WithAttributes, WithSpan},
};

use super::CodeActionsContext;

pub(super) fn add_missing_opposite_relation(
    actions: &mut Vec<CodeActionOrCommand>,
    context: &CodeActionsContext<'_>,
    field: FieldWalker<'_>,
) {
    match field.refine() {
        Some(walkers::RefinedFieldWalker::Relation(_)) => (),
        _ => return,
    }

    let name = field.model().name();
    let target_name = field.ast_field().field_type.name();
    let diagnostics = context.diagnostics_for_span_with_message(
        field.ast_field().span(),
        "is missing an opposite relation field on the model",
    );

    if diagnostics.is_empty() {
        return;
    }

    // * We know that this is safe to unwrap here as otherwise this diagnostic would be
    // * replaced with the one that we check for in `fn create_missing_block_for_model`
    let target_model = context.db.find_model(target_name).unwrap();

    let target_file_id = target_model.file_id();
    let target_file_content = context.db.source(target_file_id);

    target_model.fields().last().unwrap();

    let span = Span {
        start: target_model.ast_model().span().end - 1,
        end: target_model.ast_model().span().end - 1,
        file_id: target_file_id,
    };

    let separator = if target_model.ast_model().attributes().is_empty() {
        target_model.newline().to_string()
    } else {
        Default::default()
    };
    let indentation = target_model.indentation();
    let newline = target_model.newline();

    let formatted_content = format!("{separator}{indentation}{name} {name}[]{newline}");

    let Ok(edit) = super::create_text_edit(
        context.db.file_name(target_file_id),
        target_file_content,
        formatted_content,
        false,
        span,
    ) else {
        return;
    };

    let action = CodeAction {
        title: format!("Add missing relation field to model {}", target_name),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics: Some(diagnostics),
        ..Default::default()
    };

    actions.push(CodeActionOrCommand::CodeAction(action))
}
