use crate::pair::{DefaultKind, ModelPair};

use super::generators::{self, Warnings};

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

    for field in model.scalar_fields() {
        if let Some(DefaultKind::Prisma1Uuid) = field.default().kind() {
            let warn = generators::ModelAndField {
                model: model.name().to_string(),
                field: field.name().to_string(),
            };

            warnings.prisma_1_uuid_defaults.push(warn);
        }

        if let Some(DefaultKind::Prisma1Cuid) = field.default().kind() {
            let warn = generators::ModelAndField {
                model: model.name().to_string(),
                field: field.name().to_string(),
            };

            warnings.prisma_1_cuid_defaults.push(warn);
        }

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
                tpe: field.prisma_type().to_string(),
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
    }

    for field in model.relation_fields() {
        if field.reintrospected_relation() {
            warnings.reintrospected_relations.push(generators::Model {
                model: field.prisma_type().into_owned(),
            });
        }
    }
}
