//! Definition of warnings, which are displayed to the user during `db
//! pull`.

use introspection_connector::Warning;
use serde::Serialize;

/// Collections used for warning generation. These should be preferred
/// over directly creating warnings from the code, to prevent spamming
/// the user.
#[derive(Debug, Default)]
pub(crate) struct Warnings {
    /// Store final warnings to this vector.
    pub(crate) warnings: Vec<Warning>,
    /// Fields that are using Prisma 1 UUID defaults.
    pub(crate) prisma_1_uuid_defaults: Vec<ModelAndField>,
    /// Fields that are using Prisma 1 CUID defaults.
    pub(crate) prisma_1_cuid_defaults: Vec<ModelAndField>,
    /// Fields having an empty name.
    pub(crate) fields_with_empty_names: Vec<ModelAndField>,
    /// Field names we remapped during introspection.
    pub(crate) remapped_fields: Vec<ModelAndField>,
    /// Enum values that are empty strings.
    pub(crate) enum_values_with_empty_names: Vec<EnumAndValue>,
    /// Models that have no fields.
    pub(crate) models_without_columns: Vec<Model>,
    /// Models missing a id or unique constraint.
    pub(crate) models_without_identifiers: Vec<Model>,
    /// If the id attribute has a name taken from a previous data
    /// model.
    pub(crate) reintrospected_id_names: Vec<Model>,
    /// The field has a type we do not currently support in Prisma.
    pub(crate) unsupported_types: Vec<ModelAndFieldAndType>,
    /// The name of the model is taken from a previous data model.
    pub(crate) remapped_models: Vec<Model>,
    /// The relation is copied from a previous data model, only if
    /// `relationMode` is `prisma`.
    pub(crate) reintrospected_relations: Vec<Model>,
}

impl Warnings {
    pub(crate) fn new() -> Self {
        Self {
            warnings: Vec::new(),
            ..Default::default()
        }
    }

    pub(crate) fn push(&mut self, warning: Warning) {
        self.warnings.push(warning);
    }

    /// Generate warnings from all indicators. Must be called after
    /// introspection.
    pub(crate) fn finalize(&mut self) -> Vec<Warning> {
        fn maybe_warn<T>(elems: &[T], warning: impl Fn(&[T]) -> Warning, warnings: &mut Vec<Warning>) {
            if !elems.is_empty() {
                warnings.push(warning(elems))
            }
        }

        maybe_warn(
            &self.models_without_identifiers,
            warning_models_without_identifier,
            &mut self.warnings,
        );

        maybe_warn(&self.unsupported_types, warning_unsupported_types, &mut self.warnings);

        maybe_warn(
            &self.remapped_models,
            warning_enriched_with_map_on_model,
            &mut self.warnings,
        );

        maybe_warn(
            &self.remapped_fields,
            warning_enriched_with_map_on_field,
            &mut self.warnings,
        );

        maybe_warn(
            &self.models_without_columns,
            warning_models_without_columns,
            &mut self.warnings,
        );

        maybe_warn(
            &self.reintrospected_id_names,
            warning_enriched_with_custom_primary_key_names,
            &mut self.warnings,
        );

        maybe_warn(
            &self.prisma_1_uuid_defaults,
            warning_default_uuid_warning,
            &mut self.warnings,
        );

        maybe_warn(
            &self.prisma_1_cuid_defaults,
            warning_default_cuid_warning,
            &mut self.warnings,
        );

        maybe_warn(
            &self.enum_values_with_empty_names,
            warning_enum_values_with_empty_names,
            &mut self.warnings,
        );

        maybe_warn(
            &self.fields_with_empty_names,
            warning_fields_with_empty_names,
            &mut self.warnings,
        );

        maybe_warn(
            &self.reintrospected_relations,
            warning_relations_added_from_the_previous_data_model,
            &mut self.warnings,
        );

        std::mem::take(&mut self.warnings)
    }
}

#[derive(Serialize, Debug, Clone)]
pub(crate) struct Model {
    pub(crate) model: String,
}

#[derive(Serialize, Debug, Clone)]
pub(crate) struct Enum {
    pub(crate) enm: String,
}

impl Enum {
    pub(crate) fn new(name: &str) -> Self {
        Enum { enm: name.to_owned() }
    }
}

#[derive(Serialize, Debug, Clone)]
pub(crate) struct ModelAndField {
    pub(crate) model: String,
    pub(crate) field: String,
}

#[derive(Serialize, Debug, Clone)]
pub(crate) struct ModelAndIndex {
    pub(crate) model: String,
    pub(crate) index_db_name: String,
}

#[derive(Serialize, Debug)]
pub(crate) struct ModelAndFieldAndType {
    pub(crate) model: String,
    pub(crate) field: String,
    pub(crate) tpe: String,
}

#[derive(Serialize, Debug, Clone)]
pub(crate) struct EnumAndValue {
    pub(crate) enm: String,
    pub(crate) value: String,
}

pub(crate) fn warning_models_without_identifier(affected: &[Model]) -> Warning {
    Warning {
        code: 1,
        message: "The following models were commented out as they do not have a valid unique identifier or id. This is currently not supported by the Prisma Client.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(crate) fn warning_fields_with_empty_names(affected: &[ModelAndField]) -> Warning {
    Warning {
        code: 2,
        message: "These fields were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` attribute."
            .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(crate) fn warning_unsupported_types(affected: &[ModelAndFieldAndType]) -> Warning {
    Warning {
        code: 3,
        message: "These fields are not supported by the Prisma Client, because Prisma currently does not support their types.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(crate) fn warning_enum_values_with_empty_names(affected: &[EnumAndValue]) -> Warning {
    Warning {
        code: 4,
        message: "These enum values were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` attribute."
            .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(crate) fn warning_default_cuid_warning(affected: &[ModelAndField]) -> Warning {
    Warning {
        code: 5,
        message:
            "These id fields had a `@default(cuid())` added because we believe the schema was created by Prisma 1."
                .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(crate) fn warning_default_uuid_warning(affected: &[ModelAndField]) -> Warning {
    Warning {
        code: 6,
        message:
            "These id fields had a `@default(uuid())` added because we believe the schema was created by Prisma 1."
                .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(crate) fn warning_enriched_with_map_on_model(affected: &[Model]) -> Warning {
    Warning {
        code: 7,
        message: "These models were enriched with `@@map` information taken from the previous Prisma schema.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(crate) fn warning_enriched_with_map_on_field(affected: &[ModelAndField]) -> Warning {
    Warning {
        code: 8,
        message: "These fields were enriched with `@map` information taken from the previous Prisma schema.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(crate) fn warning_enriched_with_map_on_enum(affected: &[Enum]) -> Warning {
    Warning {
        code: 9,
        message: "These enums were enriched with `@@map` information taken from the previous Prisma schema.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(crate) fn warning_enriched_with_map_on_enum_value(affected: &[EnumAndValue]) -> Warning {
    Warning {
        code: 10,
        message: "These enum values were enriched with `@map` information taken from the previous Prisma schema."
            .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

//todo maybe we can get rid of this alltogether due to @@ignore
//but maybe we should have warnings for ignored fields and models
pub(crate) fn warning_models_without_columns(affected: &[Model]) -> Warning {
    Warning {
        code: 14,
        message: "The following models were commented out as we could not retrieve columns for them. Please check your privileges.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(crate) fn warning_enriched_with_custom_primary_key_names(affected: &[Model]) -> Warning {
    Warning {
        code: 18,
        message: "These models were enriched with custom compound id names taken from the previous Prisma schema."
            .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(crate) fn warning_relations_added_from_the_previous_data_model(affected: &[Model]) -> Warning {
    Warning {
        code: 19,
        message: "Relations were copied from the previous data model due to not using foreign keys in the database. If any of the relation columns changed in the database, the relations might not be correct anymore.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}
