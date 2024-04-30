use lsp_types::{CompletionItem, CompletionItemKind, CompletionList};
use psl::parser_database::ReferentialAction;

pub(super) fn referential_action_completion(
    completion_list: &mut CompletionList,
    referential_action: ReferentialAction,
) {
    completion_list.items.push(CompletionItem {
        label: referential_action.as_str().to_owned(),
        kind: Some(CompletionItemKind::ENUM),
        // ? (@tomhoule) what is the difference between detail and documentation?
        detail: Some(referential_action.documentation().to_owned()),
        ..Default::default()
    })
}
