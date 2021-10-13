mod names;

use crate::{diagnostics::Diagnostics, transform::ast_to_dml::db::ParserDatabase};

use self::names::Names;

pub(super) fn validate(db: &ParserDatabase<'_>, diagnostics: &mut Diagnostics) {
    let names = Names::new(db);

    for field in db.walk_models().flat_map(|m| m.relation_fields()) {
        if let Err(error) = names.validate_ambiguous_relation(field) {
            diagnostics.push_error(error);
            return;
        }
    }
}
