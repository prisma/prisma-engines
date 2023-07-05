use lsp_types::{CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams};
use psl::{
    parser_database::walkers::{EnumWalker, ModelWalker},
    schema_ast::ast::WithSpan,
    Configuration,
};

pub(super) fn add_schema_block_attribute_model(
    actions: &mut Vec<CodeActionOrCommand>,
    params: &CodeActionParams,
    schema: &str,
    config: &Configuration,
    model: ModelWalker<'_>,
) {
    let datasource = match config.datasources.first() {
        Some(ds) => ds,
        None => return,
    };

    if datasource.schemas_span.is_none() {
        return;
    }

    if model.schema_name().is_some() {
        return;
    }

    let span_diagnostics =
        match super::diagnostics_for_span(schema, &params.context.diagnostics, model.ast_model().span()) {
            Some(sd) => sd,
            None => return,
        };

    let diagnostics =
        match super::filter_diagnostics(span_diagnostics, "This model is missing an `@@schema` attribute.") {
            Some(value) => value,
            None => return,
        };

    let formatted_attribute = super::format_block_attribute(
        "schema()",
        model.indentation(),
        model.newline(),
        &model.ast_model().attributes,
    );

    let edit = super::create_text_edit(schema, formatted_attribute, true, model.ast_model().span(), params);

    let action = CodeAction {
        title: String::from("Add `@@schema` attribute"),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics: Some(diagnostics),
        ..Default::default()
    };

    actions.push(CodeActionOrCommand::CodeAction(action))
}

pub(super) fn add_schema_block_attribute_enum(
    actions: &mut Vec<CodeActionOrCommand>,
    params: &CodeActionParams,
    schema: &str,
    config: &Configuration,
    enumerator: EnumWalker<'_>,
) {
    let datasource = match config.datasources.first() {
        Some(ds) => ds,
        None => return,
    };

    if datasource.schemas_span.is_none() {
        return;
    }

    if enumerator.schema().is_some() {
        return;
    }

    let span_diagnostics =
        match super::diagnostics_for_span(schema, &params.context.diagnostics, enumerator.ast_enum().span()) {
            Some(sd) => sd,
            None => return,
        };

    let diagnostics = match super::filter_diagnostics(span_diagnostics, "This enum is missing an `@@schema` attribute.")
    {
        Some(value) => value,
        None => return,
    };

    let formatted_attribute = super::format_block_attribute(
        "schema()",
        enumerator.indentation(),
        enumerator.newline(),
        &enumerator.ast_enum().attributes,
    );

    let edit = super::create_text_edit(schema, formatted_attribute, true, enumerator.ast_enum().span(), params);

    let action = CodeAction {
        title: String::from("Add `@@schema` attribute"),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics: Some(diagnostics),
        ..Default::default()
    };

    actions.push(CodeActionOrCommand::CodeAction(action))
}
