use lsp_types::{CodeAction, CodeActionKind, CodeActionOrCommand};
use psl::{
    diagnostics::Span,
    parser_database::walkers::{EnumWalker, ModelWalker},
    schema_ast::ast::WithSpan,
};

use super::CodeActionsContext;

pub(super) fn add_namespace_block_attribute_model(
    actions: &mut Vec<CodeActionOrCommand>,
    context: &CodeActionsContext<'_>,
    model: ModelWalker<'_>,
) {
    let datasource = match context.datasource() {
        Some(ds) => ds,
        None => return,
    };

    if datasource.namespaces_span.is_none() {
        return;
    }

    if model.namespace_name().is_some() {
        return;
    }

    let file_id = model.ast_model().span().file_id;
    let file_uri = model.db.file_name(file_id);
    let file_content = model.db.source(file_id);

    let diagnostics = context.diagnostics_for_span_with_message(
        model.ast_model().span(),
        "This model is missing an `@@namespace` attribute.",
    );

    if diagnostics.is_empty() {
        return;
    }

    let formatted_attribute = super::format_block_attribute(
        "namespace()",
        model.indentation(),
        model.newline(),
        &model.ast_model().attributes,
    );

    let Ok(edit) = super::create_text_edit(
        file_uri,
        file_content,
        formatted_attribute,
        true,
        model.ast_model().span(),
    ) else {
        return;
    };

    let action = CodeAction {
        title: String::from("Add `@@namespace` attribute"),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics: Some(diagnostics),
        ..Default::default()
    };

    actions.push(CodeActionOrCommand::CodeAction(action))
}

pub(super) fn add_namespace_block_attribute_enum(
    actions: &mut Vec<CodeActionOrCommand>,
    context: &CodeActionsContext<'_>,
    enumerator: EnumWalker<'_>,
) {
    let datasource = match context.datasource() {
        Some(ds) => ds,
        None => return,
    };

    if datasource.namespaces_span.is_none() {
        return;
    }

    if enumerator.namespace().is_some() {
        return;
    }

    let file_id = enumerator.ast_enum().span().file_id;
    let file_uri = enumerator.db.file_name(file_id);
    let file_content = enumerator.db.source(file_id);

    let diagnostics = context.diagnostics_for_span_with_message(
        enumerator.ast_enum().span(),
        "This enum is missing an `@@namespace` attribute.",
    );

    if diagnostics.is_empty() {
        return;
    }

    let formatted_attribute = super::format_block_attribute(
        "namespace()",
        enumerator.indentation(),
        enumerator.newline(),
        &enumerator.ast_enum().attributes,
    );

    let Ok(edit) = super::create_text_edit(
        file_uri,
        file_content,
        formatted_attribute,
        true,
        enumerator.ast_enum().span(),
    ) else {
        return;
    };

    let action = CodeAction {
        title: String::from("Add `@@namespace` attribute"),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics: Some(diagnostics),
        ..Default::default()
    };

    actions.push(CodeActionOrCommand::CodeAction(action))
}

pub(super) fn add_namespace_to_namespaces(
    actions: &mut Vec<CodeActionOrCommand>,
    context: &CodeActionsContext<'_>,
    model: ModelWalker<'_>,
) {
    let datasource = match context.datasource() {
        Some(ds) => ds,
        None => return,
    };

    let diagnostics = context.diagnostics_for_span_with_message(
        model.ast_model().span(),
        "This namespace is not defined in the datasource.",
    );

    if diagnostics.is_empty() {
        return;
    }

    let datasource_file_id = datasource.span.file_id;
    let datasource_file_uri = context.db.file_name(datasource_file_id);
    let datasource_content = context.db.source(datasource_file_id);

    let edit = match datasource.namespaces_span {
        Some(span) => {
            let formatted_attribute = format!(r#"", "{}""#, model.namespace_name().unwrap());
            super::create_text_edit(
                datasource_file_uri,
                datasource_content,
                formatted_attribute,
                true,
                // todo: update spans so that we can just append to the end of the _inside_ of the array. Instead of needing to re-append the `]` or taking the span end -1
                Span::new(span.start, span.end - 1, psl::parser_database::FileId::ZERO),
            )
        }
        None => {
            let has_properties = datasource.provider_defined() | datasource.url_defined()
                || datasource.direct_url_defined()
                || datasource.shadow_url_defined()
                || datasource.relation_mode_defined()
                || datasource.namespaces_defined();

            let formatted_attribute = super::format_block_property(
                "namespaces",
                model.namespace_name().unwrap(),
                model.indentation(),
                model.newline(),
                has_properties,
            );

            super::create_text_edit(
                datasource_file_uri,
                datasource_content,
                formatted_attribute,
                true,
                datasource.url_span,
            )
        }
    };

    let Ok(edit) = edit else {
        return;
    };

    let action = CodeAction {
        title: String::from("Add namespace to namespaces"),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(edit),
        diagnostics: Some(diagnostics),
        ..Default::default()
    };

    actions.push(CodeActionOrCommand::CodeAction(action))
}
