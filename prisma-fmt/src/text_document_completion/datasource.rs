use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionList, Documentation, InsertTextFormat, MarkupContent, MarkupKind,
};
use psl::datamodel_connector::format_completion_docs;

pub(super) fn relation_mode_completion(completion_list: &mut CompletionList) {
    completion_list.items.push(CompletionItem {
        label: "relationMode".to_owned(),
        insert_text: Some(r#"relationMode = $0"#.to_owned()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        kind: Some(CompletionItemKind::FIELD),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format_completion_docs(
                r#"relationMode = "foreignKeys" | "prisma""#,
                r#"Set the global relation mode for all relations. Values can be either "foreignKeys" (Default), or "prisma". [Learn more](https://pris.ly/d/relation-mode)"#,
                None,
            ),
        })),
        ..Default::default()
    })
}

pub(super) fn provider_completion(completion_list: &mut CompletionList) {
    completion_list.items.push(CompletionItem {
        label: "provider".to_owned(),
        insert_text: Some(r#"provider = $0"#.to_owned()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        kind: Some(CompletionItemKind::FIELD),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format_completion_docs(
                r#"provider = "foo""#,
                r#"Describes which datasource connector to use. Can be one of the following datasource providers: `postgresql`, `mysql`, `sqlserver`, `sqlite`, `mongodb` or `cockroachdb`."#,
                None,
            ),
        })),
        ..Default::default()
    })
}
