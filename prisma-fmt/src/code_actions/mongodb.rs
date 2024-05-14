use lsp_types::{CodeAction, CodeActionKind, CodeActionOrCommand};
use psl::{parser_database::walkers::ModelWalker, schema_ast::ast::WithSpan, Datasource};

use super::CodeActionsContext;

pub(super) fn add_at_map_for_id(
    actions: &mut Vec<CodeActionOrCommand>,
    context: &CodeActionsContext<'_>,
    model: ModelWalker<'_>,
) {
    let pk = match model.primary_key() {
        Some(pk) => pk,
        None => return,
    };

    if pk.fields().len() < 1 {
        return;
    }

    let field = match pk.fields().next() {
        Some(field) => field,
        None => return,
    };

    let file_id = model.ast_model().span().file_id;
    let file_uri = model.db.file_name(file_id);
    let file_content = model.db.source(file_id);
    let diagnostics = context.diagnostics_for_span_with_message(
        model.ast_model().span(),
        r#"MongoDB model IDs must have an @map("_id") annotation."#,
    );

    if diagnostics.is_empty() {
        return;
    }

    let formatted_attribute = super::format_field_attribute(r#"@map("_id")"#);

    let Ok(edit) = super::create_text_edit(
        file_uri,
        file_content,
        formatted_attribute,
        true,
        field.ast_field().span(),
    ) else {
        return;
    };

    let action = CodeAction {
        title: r#"Add @map("_id")"#.to_owned(),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics: Some(diagnostics),
        ..Default::default()
    };

    actions.push(CodeActionOrCommand::CodeAction(action))
}

pub(super) fn add_native_for_auto_id(
    actions: &mut Vec<CodeActionOrCommand>,
    context: &CodeActionsContext<'_>,
    model: ModelWalker<'_>,
    source: &Datasource,
) {
    let pk = match model.primary_key() {
        Some(pk) => pk,
        None => return,
    };

    if pk.fields().len() < 1 {
        return;
    }

    let field = match pk.fields().next() {
        Some(field) => field,
        None => return,
    };

    let file_id = model.ast_model().span().file_id;
    let file_uri = model.db.file_name(file_id);
    let file_content = model.db.source(file_id);

    let diagnostics = context.diagnostics_for_span_with_message(
        model.ast_model().span(),
        r#"MongoDB `@default(auto())` fields must have `ObjectId` native type."#,
    );

    if diagnostics.is_empty() {
        return;
    }

    let formatted_attribute = super::format_field_attribute(format!("@{}.ObjectId", source.name).as_str());

    let Ok(edit) = super::create_text_edit(
        file_uri,
        file_content,
        formatted_attribute,
        true,
        field.ast_field().span(),
    ) else {
        return;
    };

    let action = CodeAction {
        title: r#"Add @db.ObjectId"#.to_owned(),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics: Some(diagnostics),
        ..Default::default()
    };

    actions.push(CodeActionOrCommand::CodeAction(action))
}
