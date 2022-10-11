use super::{database_name::validate_db_name, names::Names};
use crate::{
    ast::{self, WithName, WithSpan},
    datamodel_connector::RelationMode,
    diagnostics::DatamodelError,
    validate::validation_pipeline::context::Context,
};
use enumflags2::BitFlags;
use itertools::Itertools;
use parser_database::{
    walkers::{ModelWalker, RelationFieldWalker, RelationName},
    ReferentialAction,
};
use std::fmt;

struct Fields<'db> {
    fields: &'db [ast::FieldId],
    model: ModelWalker<'db>,
}

impl<'db> Fields<'db> {
    fn new(fields: &'db [ast::FieldId], model: ModelWalker<'db>) -> Self {
        Self { fields, model }
    }
}

impl<'db> fmt::Display for Fields<'db> {
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

pub(super) fn ambiguity(field: RelationFieldWalker<'_>, names: &Names<'_>) -> Result<(), DatamodelError> {
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
                field.ast_field().span(),
            ))
        }
        _ => Ok(()),
    }
}

/// Validates if the related model for the relation is ignored.
pub(super) fn ignored_related_model(field: RelationFieldWalker<'_>, ctx: &mut Context<'_>) {
    let related_model = field.related_model();
    let model = field.model();

    if !related_model.is_ignored() || field.is_ignored() || model.is_ignored() {
        return;
    }

    let message = format!(
        "The relation field `{}` on Model `{}` must specify the `@ignore` attribute, because the model {} it is pointing to is marked ignored.",
        field.name(), model.name(), related_model.name()
    );

    ctx.push_error(DatamodelError::new_attribute_validation_error(
        &message,
        "@ignore",
        field.ast_field().span(),
    ));
}

/// Does the connector support the given referential actions.
pub(super) fn referential_actions(field: RelationFieldWalker<'_>, ctx: &mut Context<'_>) {
    let connector = ctx.connector;
    let relation_mode = ctx.relation_mode;

    fn fmt_allowed_actions(allowed_actions: BitFlags<ReferentialAction>) -> String {
        allowed_actions.iter().map(|f| format!("`{}`", f.as_str())).join(", ")
    }

    // validation template for relationMode = "foreignKeys"
    let msg_foreign_keys = |action: ReferentialAction| {
        let allowed_actions = connector.referential_actions(&relation_mode);

        format!(
            "Invalid referential action: `{}`. Allowed values: ({})",
            action.as_str(),
            fmt_allowed_actions(allowed_actions),
        )
    };

    // validation template for relationMode = "prisma"
    let msg_prisma = |action: ReferentialAction| {
        let allowed_actions = connector.simulated_referential_actions(&relation_mode);

        let additional_info = match action {
            ReferentialAction::NoAction => {
                if ctx
                    .connector
                    .supports_referential_action(&relation_mode, ReferentialAction::Restrict)
                {
                    Some(format!(
                        ". `{}` is not implemented for {}, you could try using `{}`, which behaves the same if you do not need to defer constraint checks in a transaction",
                        ReferentialAction::NoAction.as_str(),
                        connector.name(),
                        ReferentialAction::Restrict.as_str(),
                    ))
                } else {
                    None
                }
            }
            _ => None,
        };
        let additional_info = additional_info.unwrap_or("".to_owned());

        format!(
            "Invalid referential action: `{}`. Allowed values: ({}){additional_info}",
            action.as_str(),
            fmt_allowed_actions(allowed_actions),
        )
    };

    let msg_template = |action: ReferentialAction| -> String {
        match relation_mode {
            RelationMode::ForeignKeys => msg_foreign_keys(action),
            RelationMode::Prisma => msg_prisma(action),
        }
    };

    if let Some(on_delete) = field.explicit_on_delete() {
        let span = field
            .ast_field()
            .span_for_argument("relation", "onDelete")
            .unwrap_or_else(|| field.ast_field().span());

        if !ctx.connector.supports_referential_action(&ctx.relation_mode, on_delete) {
            ctx.push_error(DatamodelError::new_validation_error(&msg_template(on_delete), span));
        }
    }

    if let Some(on_update) = field.explicit_on_update() {
        let span = field
            .ast_field()
            .span_for_argument("relation", "onUpdate")
            .unwrap_or_else(|| field.ast_field().span());

        if !ctx.connector.supports_referential_action(&ctx.relation_mode, on_update) {
            ctx.push_error(DatamodelError::new_validation_error(&msg_template(on_update), span));
        }
    }
}

pub(super) fn map(field: RelationFieldWalker<'_>, ctx: &mut Context<'_>) {
    if field.mapped_name().is_none() {
        return;
    }

    if !ctx.connector.supports_named_foreign_keys() {
        let span = field
            .ast_field()
            .span_for_attribute("relation")
            .unwrap_or_else(ast::Span::empty);

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            "Your provider does not support named foreign keys.",
            "@relation",
            span,
        ));
        return;
    }

    if let Some(relation_attr) = field
        .ast_field()
        .attributes
        .iter()
        .find(|attr| attr.name() == "relation")
    {
        validate_db_name(field.model().name(), relation_attr, field.mapped_name(), ctx, false);
    }
}
