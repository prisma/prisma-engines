use lsp_types::{CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams};
use psl::{
    diagnostics::Span,
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

pub(super) fn add_schema_to_schemas(
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

    let span_diagnostics =
        match super::diagnostics_for_span(schema, &params.context.diagnostics, model.ast_model().span()) {
            Some(sd) => sd,
            None => return,
        };

    let diagnostics = match super::filter_diagnostics(span_diagnostics, "This schema is not defined in the datasource.")
    {
        Some(value) => value,
        None => return,
    };

    let edit = match datasource.schemas_span {
        Some(span) => {
            let formatted_attribute = format!(r#"", "{}""#, model.schema_name().unwrap());
            super::create_text_edit(
                schema,
                formatted_attribute,
                true,
                // todo: update spans so that we can just append to the end of the _inside_ of the array. Instead of needing to re-append the `]` or taking the span end -1
                Span::new(span.start, span.end - 1, psl::parser_database::FileId::ZERO),
                params,
            )
        }
        None => {
            let has_properties = datasource.provider_defined() | datasource.url_defined()
                || datasource.direct_url_defined()
                || datasource.shadow_url_defined()
                || datasource.relation_mode_defined()
                || datasource.schemas_defined();

            let formatted_attribute = super::format_block_property(
                "schemas",
                model.schema_name().unwrap(),
                model.indentation(),
                model.newline(),
                has_properties,
            );

            super::create_text_edit(schema, formatted_attribute, true, datasource.url_span, params)
        }
    };

    let action = CodeAction {
        title: String::from("Add schema to schemas"),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics: Some(diagnostics),
        ..Default::default()
    };

    actions.push(CodeActionOrCommand::CodeAction(action))
}
