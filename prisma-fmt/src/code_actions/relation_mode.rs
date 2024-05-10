use lsp_types::{CodeAction, CodeActionKind, CodeActionOrCommand};
use psl::{
    diagnostics::FileId, parser_database::walkers::CompleteInlineRelationWalker, schema_ast::ast::SourceConfig,
    Configuration,
};

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

pub(crate) fn replace_set_default_mysql(
    actions: &mut Vec<CodeActionOrCommand>,
    params: &lsp_types::CodeActionParams,
    file_id: FileId,
    schema: &str,
    relation: CompleteInlineRelationWalker<'_>,
    config: &Configuration,
) {
    let datasource = match config.datasources.first() {
        Some(ds) => ds,
        None => return,
    };

    if datasource.active_connector.provider_name() != "mysql" {
        return;
    }

    let span = match relation.on_update_span() {
        Some(span) => span,
        None => return,
    };

    if span.file_id != file_id {
        return;
    }

    let span_diagnostics = match super::diagnostics_for_span(schema, &params.context.diagnostics, span) {
        Some(sd) => sd,
        None => return,
    };

    let diagnostics = match
        super::filter_diagnostics(
            span_diagnostics,
            "MySQL does not actually support the `SetDefault` referential action, so using it may result in unexpected errors.") {
            Some(value) => value,
            None => return,
        };

    let edit = super::create_text_edit(schema, "NoAction".to_owned(), false, span, params);

    let action = CodeAction {
        title: r#"Replace SetDefault with NoAction"#.to_owned(),

        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics: Some(diagnostics),
        ..Default::default()
    };

    actions.push(CodeActionOrCommand::CodeAction(action))
}
