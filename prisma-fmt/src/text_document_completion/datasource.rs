use std::collections::HashMap;

use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionList, Documentation, InsertTextFormat, MarkupContent, MarkupKind,
};
use psl::datamodel_connector::format_completion_docs;

use super::{add_quotes, CompletionContext};

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

pub(super) fn direct_url_completion(completion_list: &mut CompletionList) {
    completion_list.items.push(CompletionItem {
        label: "directUrl".to_owned(),
        insert_text: Some(r#"directUrl = $0"#.to_owned()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        kind: Some(CompletionItemKind::FIELD),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format_completion_docs(
                r#"directUrl = "String" | env("ENVIRONMENT_VARIABLE")"#,
                r#"Connection URL for direct connection to the database. [Learn more](https://pris.ly/d/data-proxy-cli)."#,
                None,
            )
        })),
        ..Default::default()
    })
}

pub(super) fn shadow_db_completion(completion_list: &mut CompletionList) {
    completion_list.items.push(CompletionItem {
        label: "shadowDatabaseUrl".to_owned(),
        insert_text: Some(r#"shadowDatabaseUrl = $0"#.to_owned()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        kind: Some(CompletionItemKind::FIELD),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format_completion_docs(
                r#"shadowDatabaseUrl = "String" | env("ENVIRONMENT_VARIABLE")"#,
                r#"Connection URL including authentication info to use for Migrate's [shadow database](https://pris.ly/d/migrate-shadow)."#,
                None,
            ),
        })),
        ..Default::default()
    })
}

pub(super) fn url_completion(completion_list: &mut CompletionList) {
    completion_list.items.push(CompletionItem {
        label: "url".to_owned(),
        insert_text: Some(r#"url = $0"#.to_owned()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        kind: Some(CompletionItemKind::FIELD),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format_completion_docs(
                r#"url = "String" | env("ENVIRONMENT_VARIABLE")"#,
                r#"Connection URL including authentication info. Each datasource provider documents the URL syntax. Most providers use the syntax provided by the database. [Learn more](https://pris.ly/d/connection-strings)."#,
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

pub(super) fn url_env_completion(completion_list: &mut CompletionList) {
    completion_list.items.push(CompletionItem {
        label: "env()".to_owned(),
        insert_text: Some(r#"env($0)"#.to_owned()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        kind: Some(CompletionItemKind::PROPERTY),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format_completion_docs(
                r#"env(_ environmentVariable: string)"#,
                r#"Specifies a datasource via an environment variable. When running a Prisma CLI command that needs the database connection URL (e.g. `prisma db pull`), you need to make sure that the `DATABASE_URL` environment variable is set. One way to do so is by creating a `.env` file. Note that the file must be in the same directory as your schema.prisma file to automatically be picked up by the Prisma CLI.""#,
                Some(HashMap::from([(
                    "environmentVariable",
                    "The environment variable in which the database connection URL is stored.",
                )])),
            ),
        })),
        ..Default::default()
    })
}

pub(super) fn url_quotes_completion(completion_list: &mut CompletionList) {
    completion_list.items.push(CompletionItem {
        label: r#""""#.to_owned(),
        insert_text: Some(r#""$0""#.to_owned()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        kind: Some(CompletionItemKind::PROPERTY),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format_completion_docs(
                r#""connectionString""#,
                r#"Connection URL including authentication info. Each datasource provider documents the URL syntax. Most providers use the syntax provided by the database. [Learn more](https://pris.ly/d/prisma-schema)."#,
                None,
            ),
        })),
        ..Default::default()
    })
}

pub(super) fn url_env_db_completion(completion_list: &mut CompletionList, kind: &str, ctx: CompletionContext<'_>) {
    let text = match kind {
        "url" => "DATABASE_URL",
        "directUrl" => "DIRECT_URL",
        "shadowDatabaseUrl" => "SHADOW_DATABASE_URL",
        _ => unreachable!(),
    };

    let insert_text = if add_quotes(&ctx.params, ctx.db.source(ctx.initiating_file_id)) {
        format!(r#""{text}""#)
    } else {
        text.to_owned()
    };

    completion_list.items.push(CompletionItem {
        label: text.to_owned(),
        insert_text: Some(insert_text),
        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
        kind: Some(CompletionItemKind::CONSTANT),
        ..Default::default()
    })
}
