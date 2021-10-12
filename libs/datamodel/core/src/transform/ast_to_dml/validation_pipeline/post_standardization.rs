mod names;

use crate::{diagnostics::Diagnostics, transform::ast_to_dml::db::ParserDatabase};

use self::names::Names;

pub(super) fn validate(db: &ParserDatabase<'_>, diagnostics: &mut Diagnostics) {
    let names = Names::new(db);

    for ((model_id, field_id), _) in db.types.relation_fields.iter() {
        let model = db.walk_model(*model_id);
        let field = model.relation_field(*field_id);

        if let Err(error) = names.validate_ambiguous_relation(field) {
            diagnostics.push_error(error);
            return;
        }
    }
}
