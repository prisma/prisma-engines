use std::{collections::HashMap, fmt};

use itertools::Itertools;

use crate::{
    ast::{FieldId, ModelId},
    diagnostics::DatamodelError,
    transform::ast_to_dml::db::{
        walkers::{ModelWalker, RelationFieldWalker, RelationName},
        ParserDatabase,
    },
};

type RelationIdentifier<'ast> = (ModelId, ModelId, RelationName<'ast>);

pub(super) struct Names<'ast> {
    relation_names: HashMap<RelationIdentifier<'ast>, Vec<FieldId>>,
}

impl<'ast> Names<'ast> {
    pub(super) fn new(db: &ParserDatabase<'ast>) -> Self {
        let mut relation_names: HashMap<RelationIdentifier<'ast>, Vec<FieldId>> = HashMap::new();

        for field in db.walk_models().flat_map(|m| m.relation_fields()) {
            let model_id = field.model().model_id();
            let related_model_id = field.related_model().model_id();

            let identifier = (model_id, related_model_id, field.relation_name());
            let field_ids = relation_names.entry(identifier).or_default();

            field_ids.push(field.field_id());
        }

        Self { relation_names }
    }

    pub(super) fn validate_ambiguous_relation(&self, field: RelationFieldWalker<'_, '_>) -> Result<(), DatamodelError> {
        let model = field.model();
        let related_model = field.related_model();

        let identifier = (model.model_id(), related_model.model_id(), field.relation_name());

        match self.relation_names.get(&identifier) {
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
}

struct Fields<'ast, 'db> {
    fields: &'ast [FieldId],
    model: ModelWalker<'ast, 'db>,
}

impl<'ast, 'db> Fields<'ast, 'db> {
    fn new(fields: &'ast [FieldId], model: ModelWalker<'ast, 'db>) -> Self {
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
