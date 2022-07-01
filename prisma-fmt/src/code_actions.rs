mod relations;

use std::sync::Arc;

use datamodel::{
    datamodel_connector::Diagnostics,
    parse_schema_ast,
    parser_database::{walkers::RefinedRelationWalker, ParserDatabase},
    schema_ast::source_file::SourceFile,
};
use log::warn;
use lsp_types::{CodeActionOrCommand, CodeActionParams};

pub(crate) fn empty_code_actions() -> Vec<CodeActionOrCommand> {
    Vec::new()
}

pub(crate) fn available_actions(schema: String, params: CodeActionParams) -> Vec<CodeActionOrCommand> {
    if parse_schema_ast(&schema).is_err() {
        warn!("Failed to parse schema AST in code action request.");
        return empty_code_actions();
    };

    let mut actions = Vec::new();

    let file = SourceFile::new_allocated(Arc::new(schema.into_boxed_str()));

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
