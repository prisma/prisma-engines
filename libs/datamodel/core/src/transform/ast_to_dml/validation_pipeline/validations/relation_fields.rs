use crate::{
    ast,
    diagnostics::{DatamodelError, Diagnostics},
    transform::ast_to_dml::db::walkers::{ModelWalker, RelationFieldWalker, RelationName},
};
use datamodel_connector::{Connector, ReferentialIntegrity};
use dml::relation_info::ReferentialAction;
use itertools::Itertools;
use std::fmt;

use super::names::Names;

struct Fields<'ast, 'db> {
    fields: &'ast [ast::FieldId],
    model: ModelWalker<'ast, 'db>,
}

impl<'ast, 'db> Fields<'ast, 'db> {
    fn new(fields: &'ast [ast::FieldId], model: ModelWalker<'ast, 'db>) -> Self {
        Self { fields, model }
    }
}

impl<'ast, 'db> fmt::Display for Fields<'ast, 'db> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fields = self
            .fields
            .iter()
            .map(|field_id| self.model.relation_field(*field_id).name())
            .map(|name| format!("`{}`", name));

        match fields.len() {
            x if x < 2 => f.write_str(&fields.join(", ")),
            2 => f.write_str(&fields.join(" and ")),
            _ => {
                let len = fields.len();

                for (i, name) in fields.enumerate() {
                    f.write_str(&name)?;

                    if i < len - 2 {
                        f.write_str(", ")?;
                    } else if i < len - 1 {
                        f.write_str(" and ")?;
                    }
                }

                Ok(())
            }
        }
    }
}

pub(super) fn ambiguity(field: RelationFieldWalker<'_, '_>, names: &Names<'_>) -> Result<(), DatamodelError> {
    let model = field.model();
    let related_model = field.related_model();

    let identifier = (model.model_id(), related_model.model_id(), field.relation_name());

    match names.relation_names.get(&identifier) {
        Some(fields) if fields.len() > 1 => {
            let field_names = Fields::new(fields, model);
            let relation_name = identifier.2;
            let is_self_relation = model == related_model;

            let message = match relation_name {
                RelationName::Generated(_) if is_self_relation && fields.len() == 2 => {
                    format!(
                        "Ambiguous self relation detected. The fields {} in model `{}` both refer to `{}`. If they are part of the same relation add the same relation name for them with `@relation(<name>)`.",
                        field_names,
                        model.name(),
                        related_model.name(),
                    )
                }
                RelationName::Generated(_) if is_self_relation && fields.len() > 2 => {
                    format!(
                        "Unnamed self relation detected. The fields {} in model `{}` have no relation name. Please provide a relation name for one of them by adding `@relation(<name>).",
                        field_names,
                        model.name(),
                    )
                }
                RelationName::Explicit(_) if is_self_relation && fields.len() > 2 => {
                    format!(
                        "Wrongly named self relation detected. The fields {} in model `{}` have the same relation name. At most two relation fields can belong to the same relation and therefore have the same name. Please assign a different relation name to one of them.",
                        field_names,
                        model.name(),
                    )
                }
                RelationName::Explicit(_) if is_self_relation && fields.len() == 2 => return Ok(()),
                RelationName::Generated(_) => {
                    format!(
                        "Ambiguous relation detected. The fields {} in model `{}` both refer to `{}`. Please provide different relation names for them by adding `@relation(<name>).",
                        field_names,
                        model.name(),
                        related_model.name(),
                    )
                }
                RelationName::Explicit(_) => {
                    format!(
                        "Wrongly named relation detected. The fields {} in model `{}` both use the same relation name. Please provide different relation names for them through `@relation(<name>).",
                        field_names,
                        model.name(),
                    )
                }
            };

            Err(DatamodelError::new_model_validation_error(
                &message,
                model.name(),
                field.ast_field().span,
            ))
        }
        _ => Ok(()),
    }
}

/// Validates usage of `onUpdate` with the `referentialIntegrity` set to
/// `prisma`.
///
/// This is temporary to the point until Query Engine supports `onUpdate`
/// actions on emulations.
pub(super) fn on_update_without_foreign_keys(
    field: RelationFieldWalker<'_, '_>,
    referential_integrity: ReferentialIntegrity,
    diagnostics: &mut Diagnostics,
) {
    if referential_integrity.uses_foreign_keys() {
        return;
    }

    if field
        .attributes()
        .on_update
        .filter(|act| *act != ReferentialAction::NoAction)
        .is_none()
    {
        return;
    }

    let ast_field = field.ast_field();

    let span = ast_field
        .span_for_argument("relation", "onUpdate")
        .unwrap_or(ast_field.span);

    diagnostics.push_error(DatamodelError::new_validation_error(
        "Referential actions other than `NoAction` will not work for `onUpdate` without foreign keys. Please follow the issue: https://github.com/prisma/prisma/issues/9014",
        span
    ));
}

/// Validates if the related model for the relation is ignored.
pub(super) fn ignored_related_model(field: RelationFieldWalker<'_, '_>, diagnostics: &mut Diagnostics) {
    let related_model = field.related_model();
    let model = field.model();

    if !related_model.attributes().is_ignored || field.attributes().is_ignored || model.attributes().is_ignored {
        return;
    }

    let message = format!(
        "The relation field `{}` on Model `{}` must specify the `@ignore` attribute, because the model {} it is pointing to is marked ignored.",
        field.name(), model.name(), related_model.name()
    );

    diagnostics.push_error(DatamodelError::new_attribute_validation_error(
        &message,
        "ignore",
        field.ast_field().span,
    ));
}

/// Does the connector support the given referential actions.
pub(super) fn referential_actions(
    field: RelationFieldWalker<'_, '_>,
    connector: &dyn Connector,
    diagnostics: &mut Diagnostics,
) {
    let msg = |action| {
        let allowed_values = connector
            .referential_actions()
            .iter()
            .map(|f| format!("`{}`", f))
            .join(", ");

        format!(
            "Invalid referential action: `{}`. Allowed values: ({})",
            action, allowed_values,
        )
    };

    if let Some(on_delete) = field.attributes().on_delete {
        if !connector.supports_referential_action(on_delete) {
            let span = field
                .ast_field()
                .span_for_argument("relation", "onDelete")
                .unwrap_or_else(|| field.ast_field().span);

            diagnostics.push_error(DatamodelError::new_validation_error(&msg(on_delete), span));
        }
    }

    if let Some(on_update) = field.attributes().on_update {
        if !connector.supports_referential_action(on_update) {
            let span = field
                .ast_field()
                .span_for_argument("relation", "onUpdate")
                .unwrap_or_else(|| field.ast_field().span);

            diagnostics.push_error(DatamodelError::new_validation_error(&msg(on_update), span));
        }
    }
}
