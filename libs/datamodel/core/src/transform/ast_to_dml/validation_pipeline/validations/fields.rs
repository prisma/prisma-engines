use super::names::{NameTaken, Names};
use crate::{
    diagnostics::{DatamodelError, Diagnostics},
    transform::ast_to_dml::db::walkers::FieldWalker,
};

pub(super) fn validate_name(field: FieldWalker<'_, '_>, names: &Names<'_>, diagnostics: &mut Diagnostics) {
    let model = field.model();

    match names.name_taken(model.model_id(), field.name()) {
        Some(NameTaken::Index) => {
            let message = format!(
                "The custom name `{}` specified for the `@@index` attribute is already used as a name for a field. Please choose a different name.",
                field.name()
            );

            let error = DatamodelError::new_model_validation_error(&message, model.name(), model.ast_model().span);
            diagnostics.push_error(error);
        }
        Some(NameTaken::Unique) => {
            let message = format!(
                "The custom name `{}` specified for the `@@unique` attribute is already used as a name for a field. Please choose a different name.",
                field.name()
            );

            let error = DatamodelError::new_model_validation_error(&message, model.name(), model.ast_model().span);
            diagnostics.push_error(error);
        }
        Some(NameTaken::PrimaryKey) => {
            let message = format!(
                "The custom name `{}` specified for the `@@id` attribute is already used as a name for a field. Please choose a different name.",
                field.name()
            );

            let error = DatamodelError::new_model_validation_error(&message, model.name(), model.ast_model().span);
            diagnostics.push_error(error);
        }
        None => (),
    }
}
