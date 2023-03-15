use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionList, Documentation, InsertTextFormat, MarkupContent, MarkupKind,
};
use psl_core::datamodel_connector::format_completion_docs;

pub(crate) fn extensions_completion(completion_list: &mut CompletionList) {
    completion_list.items.push(CompletionItem {
        label: "extensions".to_owned(),
        insert_text: Some("extensions = [$0]".to_owned()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        kind: Some(CompletionItemKind::FIELD),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format_completion_docs(
                r#"extensions = [pg_trgm, postgis(version: "2.1")]"#,
                r#"Enable PostgreSQL extensions. [Learn more](https://pris.ly/d/postgresql-extensions)"#,
                None,
            ),
        })),
        ..Default::default()
    })
}

pub(crate) fn schemas_completion(completion_list: &mut CompletionList) {
    completion_list.items.push(CompletionItem {
        label: "schemas".to_owned(),
        insert_text: Some(r#"schemas = [$0]"#.to_owned()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        kind: Some(CompletionItemKind::FIELD),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format_completion_docs(
                r#"schemas = ["foo", "bar", "baz"]"#,
                "The list of database schemas. [Learn More](https://pris.ly/d/multi-schema-configuration)",
                None,
            ),
        })),
        // detail: Some("schemas".to_owned()),
        ..Default::default()
    });
}
