use lsp_types::{CodeAction, CodeActionKind, CodeActionOrCommand};
use psl::schema_ast::ast::SourceConfig;

pub(crate) fn edit_referential_integrity(
    actions: &mut Vec<CodeActionOrCommand>,
    params: &lsp_types::CodeActionParams,
    schema: &str,
    source: &SourceConfig,
) {
    let prop = match source.properties.iter().find(|p| p.name.name == "referentialIntegrity") {
        Some(prop) => prop,
        None => return,
    };

    let span_diagnostics = match super::diagnostics_for_span(schema, &params.context.diagnostics, source.span) {
        Some(sd) => sd,
        None => return,
    };

    let diagnostics =
        match super::filter_diagnostics(span_diagnostics, "The `referentialIntegrity` attribute is deprecated.") {
            Some(value) => value,
            None => return,
        };

    let edit = super::create_text_edit(schema, "relationMode".to_owned(), false, prop.name.span, params);

    let action = CodeAction {
        title: String::from("Rename property to relationMode"),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics: Some(diagnostics),
        ..Default::default()
    };

    actions.push(CodeActionOrCommand::CodeAction(action))
}
