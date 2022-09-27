mod relations;

use lsp_types::{CodeActionOrCommand, CodeActionParams, Diagnostic};
use psl::{
    parser_database::{ast, walkers::RefinedRelationWalker, ParserDatabase, SourceFile},
    Diagnostics,
};
use std::sync::Arc;

pub(crate) fn empty_code_actions() -> Vec<CodeActionOrCommand> {
    Vec::new()
}

pub(crate) fn available_actions(schema: String, params: CodeActionParams) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();

    let file = SourceFile::new_allocated(Arc::from(schema.into_boxed_str()));

    let db = {
        let mut diag = Diagnostics::new();
        ParserDatabase::new(file.clone(), &mut diag)
    };

    for relation in db.walk_relations() {
        if let RefinedRelationWalker::Inline(relation) = relation.refine() {
            let complete_relation = match relation.as_complete() {
                Some(relation) => relation,
                None => continue,
            };

            relations::add_referenced_side_unique(&mut actions, &params, file.as_str(), complete_relation);

            if relation.is_one_to_one() {
                relations::add_referencing_side_unique(&mut actions, &params, file.as_str(), complete_relation);
            }
        }
    }

    actions
}

/// A function to find diagnostics matching the given span. Used for
/// copying the diagnostics to a code action quick fix.
pub(super) fn diagnostics_for_span(
    schema: &str,
    diagnostics: &[Diagnostic],
    span: ast::Span,
) -> Option<Vec<Diagnostic>> {
    let res: Vec<_> = diagnostics
        .iter()
        .filter(|diag| span.overlaps(crate::range_to_span(diag.range, schema)))
        .cloned()
        .collect();

    if res.is_empty() {
        None
    } else {
        Some(res)
    }
}
