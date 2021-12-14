mod actions;
mod lint;
mod native;
mod preview;
mod text_document_completion;

/// The API is modelled on an LSP [completion
/// request](https://github.com/microsoft/language-server-protocol/blob/gh-pages/_specifications/specification-3-16.md#textDocument_completion).
/// Input and output are both JSON, the request being a `CompletionParams` object and the response
/// being a `CompletionList` object.
pub fn text_document_completion(schema: &str, params: String) -> String {
    let params = if let Ok(params) = serde_json::from_str::<lsp_types::CompletionParams>(&params) {
        params
    } else {
        return serde_json::to_string(&text_document_completion::empty_completion_list()).unwrap();
    };

    let completion_list = text_document_completion::completion(schema, params);

    serde_json::to_string(&completion_list).unwrap()
}

pub fn format(schema: String) -> String {
    use datamodel::ast::reformat::Reformatter;
    Reformatter::new(&schema).reformat_to_string()
}

pub fn lint(schema: String) -> String {
    lint::run(&schema)
}

pub fn native_types(schema: String) -> String {
    native::run(&schema)
}

pub fn preview_features() -> String {
    preview::run()
}

pub fn referential_actions(schema: String) -> String {
    actions::run(&schema)
}
