mod relations;

use datamodel::{
    datamodel_connector::Diagnostics,
    parse_schema_ast,
    parser_database::{walkers::RefinedRelationWalker, ParserDatabase},
};
use log::warn;
use lsp_types::{CodeActionOrCommand, CodeActionParams};

pub(crate) fn empty_code_actions() -> Vec<CodeActionOrCommand> {
    Vec::new()
}

pub(crate) fn available_actions(schema: &str, params: CodeActionParams) -> Vec<CodeActionOrCommand> {
    let schema_ast = if let Ok(schema_ast) = parse_schema_ast(schema) {
        schema_ast
    } else {
        warn!("Failed to parse schema AST in code action request.");
        return empty_code_actions();
    };

    let mut actions = Vec::new();

    let db = {
        let mut diag = Diagnostics::new();
        ParserDatabase::new(schema_ast, &mut diag)
    };

    for relation in db.walk_relations() {
        if let RefinedRelationWalker::Inline(relation) = relation.refine() {
            let complete_relation = match relation.as_complete() {
                Some(relation) => relation,
                None => continue,
            };

            relations::add_referenced_side_unique(&mut actions, &params, schema, complete_relation);

            if relation.is_one_to_one() {
                relations::add_referencing_side_unique(&mut actions, &params, schema, complete_relation);
            }
        }
    }

    actions
}
