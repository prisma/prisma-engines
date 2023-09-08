use lsp_types::{CodeAction, CodeActionKind, CodeActionOrCommand};
use psl::{parser_database::walkers::ModelWalker, schema_ast::ast::WithSpan};

pub(super) fn add_at_map_for_id(
    actions: &mut Vec<CodeActionOrCommand>,
    params: &lsp_types::CodeActionParams,
    schema: &str,
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

    let span_diagnostics =
        match super::diagnostics_for_span(schema, &params.context.diagnostics, model.ast_model().span()) {
            Some(sd) => sd,
            None => return,
        };

    let diagnostics = match super::filter_diagnostics(
        span_diagnostics,
        r#"MongoDB model IDs must have an @map("_id") annotation."#,
    ) {
        Some(value) => value,
        None => return,
    };

    let formatted_attribute = super::format_field_attribute(r#"@map("_id")"#);

    let edit = super::create_text_edit(schema, formatted_attribute, true, field.ast_field().span(), params);

    let action = CodeAction {
        title: r#"Add @map("_id")"#.to_owned(),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics: Some(diagnostics),
        ..Default::default()
    };

    actions.push(CodeActionOrCommand::CodeAction(action))
}
