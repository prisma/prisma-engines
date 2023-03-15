use std::collections::HashMap;

/// Formats the documentation for a completion.
/// example: How the completion is expected to be used.
///
/// # Example
///
/// ```
/// use psl_core::datamodel_connector::format_completion_docs;
///
/// let doc = format_completion_docs(
///     r#"relationMode = "foreignKeys" | "prisma""#,
///     r#"Sets the global relation mode for relations."#,
///     None,
/// );
///
/// assert_eq!(
///     "```prisma\nrelationMode = \"foreignKeys\" | \"prisma\"\n```\n___\nSets the global relation mode for relations.\n\n",
///     &doc
/// );
/// ```
pub fn format_completion_docs(example: &str, description: &str, params: Option<HashMap<&str, &str>>) -> String {
    let param_docs: String = match params {
        Some(params) => params
            .into_iter()
            .map(|(param_label, param_doc)| format!("_@param_ {param_label} {param_doc}"))
            .collect::<Vec<String>>()
            .join("\n"),
        None => Default::default(),
    };

    format!("```prisma\n{example}\n```\n___\n{description}\n\n{param_docs}")
}
