use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionList, Documentation, InsertTextFormat, MarkupContent, MarkupKind,
};

use super::generate_pretty_doc;

pub(super) fn schemas_completion(completion_list: &mut CompletionList) {
    completion_list.items.push(CompletionItem {
        label: "engines schemas".to_owned(),
        insert_text: Some(r#"schemas = [$0]"#.to_owned()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        kind: Some(CompletionItemKind::FIELD),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: generate_pretty_doc(
                r#"schemas = ["foo", "bar", "baz"]"#,
                "The list of database schemas. [Learn More](https://pris.ly/d/multi-schema-configuration)",
            ),
        })),
        // detail: Some("schemas".to_owned()),
        ..Default::default()
    });
}

pub(super) fn relation_mode_completion(completion_list: &mut CompletionList) {
    completion_list.items.push(CompletionItem {
        label: "engines relationMode".to_owned(),
        insert_text: Some(r#"relationmode = $0"#.to_owned()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        kind: Some(CompletionItemKind::FIELD),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: generate_pretty_doc(
                r#"relationMode = "foreignKeys" | "prisma""#,
                r#"Set the global relation mode for all relations. Values can be either "foreignKeys" (Default), or "prisma". [Learn more](https://pris.ly/d/relation-mode)"#
            ),
        })),
        ..Default::default()
    })
}

pub(super) fn direct_url_completion(completion_list: &mut CompletionList) {
    completion_list.items.push(CompletionItem {
        label: "engines directUrl".to_owned(),
        insert_text: Some(r#"directUrl = $0"#.to_owned()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        kind: Some(CompletionItemKind::FIELD),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: generate_pretty_doc(
                r#"directUrl = "String" | env("ENVIRONMENT_VARIABLE")"#,
                r#"Connection URL for direct connection to the database. [Learn more](https://pris.ly/d/data-proxy-cli)."#
            )
        })),
        ..Default::default()
    })
}

pub(super) fn shadow_db_completion(completion_list: &mut CompletionList) {
    completion_list.items.push(CompletionItem {
        label: "engines shadowDatabaseUrl".to_owned(),
        insert_text: Some(r#"shadowDatabaseUrl = $0"#.to_owned()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        kind: Some(CompletionItemKind::FIELD),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: generate_pretty_doc(
                r#"shadowDatabaseUrl = "String" | env("ENVIRONMENT_VARIABLE")"#,
                r#"Connection URL including authentication info to use for Migrate's [shadow database](https://pris.ly/d/migrate-shadow)."#,
            ),
        })),
        ..Default::default()
    })
}

pub(super) fn url_completion(completion_list: &mut CompletionList) {
    completion_list.items.push(CompletionItem {
        label: "engines url".to_owned(),
        insert_text: Some(r#"url = $0"#.to_owned()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        kind: Some(CompletionItemKind::FIELD),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: generate_pretty_doc(
                r#"url = "String" | env("ENVIRONMENT_VARIABLE")"#,
                r#"Connection URL including authentication info. Each datasource provider documents the URL syntax. Most providers use the syntax provided by the database. [Learn more](https://pris.ly/d/connection-strings)."#,
            ),
        })),
        ..Default::default()
    })
}

pub(super) fn provider_completion(completion_list: &mut CompletionList) {
    completion_list.items.push(CompletionItem {
        label: "engines provider".to_owned(),
        insert_text: Some(r#"provider = $0"#.to_owned()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        kind: Some(CompletionItemKind::FIELD),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: generate_pretty_doc(
                r#"provider = "foo""#,
                r#"Describes which datasource connector to use. Can be one of the following datasource providers: `postgresql`, `mysql`, `sqlserver`, `sqlite`, `mongodb` or `cockroachdb`."#,
            ),
        })),
        ..Default::default()
    })
}
