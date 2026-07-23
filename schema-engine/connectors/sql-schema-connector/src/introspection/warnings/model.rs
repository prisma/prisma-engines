use crate::introspection::introspection_pair::ModelPair;
use schema_connector::{Warnings, warnings as generators};

/// Analyze and generate warnigs from a model.
pub(super) fn generate_warnings(model: ModelPair<'_>, warnings: &mut Warnings) {
    if model.id().and_then(|id| id.name()).is_some() {
        warnings.reintrospected_id_names_in_model.push(generators::Model {
            model: model.name().to_string(),
        });
    }

    if model.scalar_fields().len() == 0 {
        warnings.models_without_columns.push(generators::Model {
            model: model.name().to_string(),
        });
    } else if !model.has_usable_identifier() && !model.ignored_in_psl() {
        warnings.models_without_identifiers.push(generators::Model {
            model: model.name().to_string(),
        });
    }

    if model.is_partition() {
        warnings.partition_tables.push(generators::Model {
            model: model.name().to_string(),
        });
    }

    if model.has_subclass() {
        warnings.inherited_tables.push(generators::Model {
            model: model.name().to_string(),
        });
    }

    if model.has_row_level_security() {
        warnings.row_level_security_tables.push(generators::Model {
            model: model.name().to_string(),
        });
    }

    if model.uses_duplicate_name() {
        warnings.duplicate_names.push(generators::TopLevelItem {
            r#type: generators::TopLevelType::Model,
            name: model.name().to_string(),
        })
    }

    if model.remapped_name() {
        warnings.remapped_models.push(generators::Model {
            model: model.name().to_string(),
        });
    }

    if model.description().is_some() {
        warnings.objects_with_comments.push(generators::Object {
            r#type: "model",
            name: model.name().to_string(),
        })
    }

    for constraint in model.check_constraints() {
        warnings.check_constraints.push(generators::ModelAndConstraint {
            model: model.name().to_string(),
            constraint: constraint.to_string(),
        })
    }

    for expr_indx in model.expression_indexes() {
        warnings.expression_indexes.push(generators::ModelAndConstraint {
            model: model.name().to_string(),
            constraint: expr_indx.to_string(),
        })
    }

    for field in model.scalar_fields() {
        if field.remapped_name_from_psl() {
            let mf = generators::ModelAndField {
                model: model.name().to_string(),
                field: field.name().to_string(),
            };

            warnings.remapped_fields_in_model.push(mf);
        }

        if field.is_unsupported() {
            let mf = generators::ModelAndFieldAndType {
                model: model.name().to_string(),
                field: field.name().to_string(),
                r#type: field.prisma_type().to_string(),
            };

            warnings.unsupported_types_in_model.push(mf)
        }

        if field.remapped_name_empty() {
            let mf = generators::ModelAndField {
                model: model.name().to_string(),
                field: field.name().to_string(),
            };

            warnings.fields_with_empty_names_in_model.push(mf);
        }

        if field.description().is_some() {
            warnings.objects_with_comments.push(generators::Object {
                r#type: "field",
                name: format!("{}.{}", model.name(), field.name()),
            })
        }
    }

    for field in model.relation_fields() {
        if field.reintrospected_relation() {
            warnings.reintrospected_relations.push(generators::Model {
                model: field.prisma_type().into_owned(),
            });
        }
    }
}
