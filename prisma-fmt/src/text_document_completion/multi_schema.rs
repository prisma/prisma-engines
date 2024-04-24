use lsp_types::{CompletionItem, CompletionItemKind, CompletionList};

pub(super) fn schema_namespace_completion(
    completion_list: &mut CompletionList,
    namespace: &String,
    insert_text: String,
) {
    completion_list.items.push(CompletionItem {
        label: String::from(namespace),
        insert_text: Some(insert_text),
        kind: Some(CompletionItemKind::PROPERTY),
        ..Default::default()
    })
}
