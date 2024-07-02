use lsp_types::{CodeAction, CodeActionKind, CodeActionOrCommand};
use psl::{
    parser_database::walkers::CompleteInlineRelationWalker,
    schema_ast::ast::{SourceConfig, WithIdentifier, WithName},
};

use super::CodeActionsContext;

pub(crate) fn edit_referential_integrity(
    actions: &mut Vec<CodeActionOrCommand>,
    context: &CodeActionsContext<'_>,
    source: &SourceConfig,
) {
    let prop = match source.properties.iter().find(|p| p.name() == "referentialIntegrity") {
        Some(prop) => prop,
        None => return,
    };

    let diagnostics =
        context.diagnostics_for_span_with_message(source.span, "The `referentialIntegrity` attribute is deprecated.");

    let Ok(edit) = super::create_text_edit(
        context.initiating_file_uri(),
        context.initiating_file_source(),
        "relationMode".to_owned(),
        false,
        prop.identifier().span,
    ) else {
        return;
    };

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
    context: &CodeActionsContext<'_>,
    relation: CompleteInlineRelationWalker<'_>,
) {
    let datasource = match context.datasource() {
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

    if span.file_id != context.initiating_file_id {
        return;
    }

    let file_name = context.initiating_file_uri();
    let file_content = context.initiating_file_source();

    let diagnostics =
        context.diagnostics_for_span_with_message(
            span,
            "MySQL does not actually support the `SetDefault` referential action, so using it may result in unexpected errors."
        );

    if diagnostics.is_empty() {
        return;
    }

    let Ok(edit) = super::create_text_edit(file_name, file_content, "NoAction".to_owned(), false, span) else {
        return;
    };

    let action = CodeAction {
        title: r#"Replace SetDefault with NoAction"#.to_owned(),

        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics: Some(diagnostics),
        ..Default::default()
    };

    actions.push(CodeActionOrCommand::CodeAction(action))
}
