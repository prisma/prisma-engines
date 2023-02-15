use introspection_connector::Warning;
use serde::Serialize;

/// Collections used for warning generation. These should be preferred
/// over directly creating warnings from the code, to prevent spamming
/// the user.
#[derive(Debug, Default)]
pub(super) struct Warnings {
    /// Store final warnings to this vector.
    warnings: Vec<Warning>,
    /// Fields that are using Prisma 1 UUID defaults.
    pub(super) prisma_1_uuid_defaults: Vec<ModelAndField>,
    /// Fields that are using Prisma 1 CUID defaults.
    pub(super) prisma_1_cuid_defaults: Vec<ModelAndField>,
    /// Fields having an empty name.
    pub(super) fields_with_empty_names_in_model: Vec<ModelAndField>,
    /// Fields having an empty name.
    pub(super) fields_with_empty_names_in_view: Vec<ViewAndField>,
    /// Field names in models we remapped during introspection.
    pub(super) remapped_fields_in_model: Vec<ModelAndField>,
    /// Field names in views we remapped during introspection.
    pub(super) remapped_fields_in_view: Vec<ViewAndField>,
    /// Enum values that are empty strings.
    pub(super) enum_values_with_empty_names: Vec<EnumAndValue>,
    /// Models that have no fields.
    pub(super) models_without_columns: Vec<Model>,
    /// Models missing a id or unique constraint.
    pub(super) models_without_identifiers: Vec<Model>,
    /// Views missing a id or unique constraint.
    pub(super) views_without_identifiers: Vec<View>,
    /// If the id attribute has a name taken from a previous model.
    pub(super) reintrospected_id_names_in_model: Vec<Model>,
    /// If the id attribute has a name taken from a previous view.
    pub(super) reintrospected_id_names_in_view: Vec<View>,
    /// The field in model has a type we do not currently support in Prisma.
    pub(super) unsupported_types_in_model: Vec<ModelAndFieldAndType>,
    /// The field in view has a type we do not currently support in Prisma.
    pub(super) unsupported_types_in_view: Vec<ViewAndFieldAndType>,
    /// The name of the model is taken from a previous data model.
    pub(super) remapped_models: Vec<Model>,
    /// The name of the model is taken from a previous data model.
    pub(super) remapped_views: Vec<View>,
    /// The name of the enum variant is taken from a previous data model.
    pub(super) remapped_values: Vec<EnumAndValue>,
    /// The relation is copied from a previous data model, only if
    /// `relationMode` is `prisma`.
    pub(super) reintrospected_relations: Vec<Model>,
    /// The name of these models or enums was a dupe in the PSL.
    pub(super) duplicate_names: Vec<TopLevelItem>,
    /// Warn about using partition tables, which only have introspection support.
    pub(super) partition_tables: Vec<Model>,
    /// Warn about using inherited tables, which only have introspection support.
    pub(super) inherited_tables: Vec<Model>,
}

impl Warnings {
    pub(super) fn new() -> Self {
        Self {
            warnings: Vec::new(),
            ..Default::default()
        }
    }

    pub(super) fn push(&mut self, warning: Warning) {
        self.warnings.push(warning);
    }

    /// Generate warnings from all indicators. Must be called after
    /// introspection.
    pub(super) fn finalize(mut self) -> Vec<Warning> {
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

        maybe_warn(
            &self.views_without_identifiers,
            warning_views_without_identifier,
            &mut self.warnings,
        );

        maybe_warn(
            &self.unsupported_types_in_model,
            warning_unsupported_types_in_models,
            &mut self.warnings,
        );

        maybe_warn(
            &self.unsupported_types_in_view,
            warning_unsupported_types_in_views,
            &mut self.warnings,
        );

        maybe_warn(
            &self.remapped_models,
            warning_enriched_with_map_on_model,
            &mut self.warnings,
        );

        maybe_warn(
            &self.remapped_values,
            warning_enriched_with_map_on_enum_value,
            &mut self.warnings,
        );

        maybe_warn(
            &self.remapped_views,
            warning_enriched_with_map_on_view,
            &mut self.warnings,
        );

        maybe_warn(
            &self.remapped_fields_in_model,
            warning_enriched_with_map_on_field_in_models,
            &mut self.warnings,
        );

        maybe_warn(
            &self.remapped_fields_in_view,
            warning_enriched_with_map_on_field_in_views,
            &mut self.warnings,
        );

        maybe_warn(
            &self.models_without_columns,
            warning_models_without_columns,
            &mut self.warnings,
        );

        maybe_warn(
            &self.reintrospected_id_names_in_model,
            warning_enriched_with_custom_primary_key_names_in_models,
            &mut self.warnings,
        );

        maybe_warn(
            &self.reintrospected_id_names_in_view,
            warning_enriched_with_custom_primary_key_names_in_views,
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
            &self.fields_with_empty_names_in_model,
            warning_fields_with_empty_names_in_models,
            &mut self.warnings,
        );

        maybe_warn(
            &self.fields_with_empty_names_in_view,
            warning_fields_with_empty_names_in_views,
            &mut self.warnings,
        );

        maybe_warn(
            &self.reintrospected_relations,
            warning_relations_added_from_the_previous_data_model,
            &mut self.warnings,
        );

        maybe_warn(
            &self.duplicate_names,
            warning_top_level_item_name_is_a_dupe,
            &mut self.warnings,
        );

        maybe_warn(&self.partition_tables, partition_tables_found, &mut self.warnings);

        maybe_warn(&self.inherited_tables, inherited_tables_found, &mut self.warnings);

        self.warnings
    }
}

#[derive(Serialize, Debug, Clone)]
pub(super) struct Model {
    pub(super) model: String,
}

#[derive(Serialize, Debug, Clone)]
pub(super) struct View {
    pub(super) view: String,
}

#[derive(Serialize, Debug, Clone)]
pub(super) struct Enum {
    pub(super) enm: String,
}

impl Enum {
    pub(super) fn new(name: &str) -> Self {
        Enum { enm: name.to_owned() }
    }
}

#[derive(Serialize, Debug, Clone)]
pub(super) struct ModelAndField {
    pub(super) model: String,
    pub(super) field: String,
}

#[derive(Serialize, Debug, Clone)]
pub(super) struct ViewAndField {
    pub(super) view: String,
    pub(super) field: String,
}

#[derive(Serialize, Debug, Clone)]
pub(super) struct ModelAndIndex {
    pub(super) model: String,
    pub(super) index_db_name: String,
}

#[derive(Serialize, Debug)]
pub(super) struct ModelAndFieldAndType {
    pub(super) model: String,
    pub(super) field: String,
    pub(super) tpe: String,
}

#[derive(Serialize, Debug)]
pub(super) struct ViewAndFieldAndType {
    pub(super) view: String,
    pub(super) field: String,
    pub(super) tpe: String,
}

#[derive(Serialize, Debug, Clone)]
pub(super) struct EnumAndValue {
    pub(super) enm: String,
    pub(super) value: String,
}

#[derive(Serialize, Debug, Clone, Copy)]
pub(super) enum TopLevelType {
    Model,
    Enum,
    View,
}

#[derive(Serialize, Debug, Clone)]
pub(super) struct TopLevelItem {
    pub(super) r#type: TopLevelType,
    pub(super) name: String,
}

pub(super) fn warning_models_without_identifier(affected: &[Model]) -> Warning {
    Warning {
        code: 1,
        message: "The following models were ignored as they do not have a valid unique identifier or id. This is currently not supported by the Prisma Client.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(super) fn warning_fields_with_empty_names_in_models(affected: &[ModelAndField]) -> Warning {
    Warning {
        code: 2,
        message: "These fields were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` attribute."
            .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(super) fn warning_unsupported_types_in_models(affected: &[ModelAndFieldAndType]) -> Warning {
    Warning {
        code: 3,
        message: "These fields are not supported by the Prisma Client, because Prisma currently does not support their types.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(super) fn warning_enum_values_with_empty_names(affected: &[EnumAndValue]) -> Warning {
    Warning {
        code: 4,
        message: "These enum values were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` attribute."
            .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(super) fn warning_default_cuid_warning(affected: &[ModelAndField]) -> Warning {
    Warning {
        code: 5,
        message:
            "These id fields had a `@default(cuid())` added because we believe the schema was created by Prisma 1."
                .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(super) fn warning_default_uuid_warning(affected: &[ModelAndField]) -> Warning {
    Warning {
        code: 6,
        message:
            "These id fields had a `@default(uuid())` added because we believe the schema was created by Prisma 1."
                .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(super) fn warning_enriched_with_map_on_model(affected: &[Model]) -> Warning {
    Warning {
        code: 7,
        message: "These models were enriched with `@@map` information taken from the previous Prisma schema.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(super) fn warning_enriched_with_map_on_field_in_models(affected: &[ModelAndField]) -> Warning {
    Warning {
        code: 8,
        message: "These fields were enriched with `@map` information taken from the previous Prisma schema.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(super) fn warning_enriched_with_map_on_enum(affected: &[Enum]) -> Warning {
    Warning {
        code: 9,
        message: "These enums were enriched with `@@map` information taken from the previous Prisma schema.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(super) fn warning_enriched_with_map_on_enum_value(affected: &[EnumAndValue]) -> Warning {
    Warning {
        code: 10,
        message: "These enum values were enriched with `@map` information taken from the previous Prisma schema."
            .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

//todo maybe we can get rid of this alltogether due to @@ignore
//but maybe we should have warnings for ignored fields and models
pub(super) fn warning_models_without_columns(affected: &[Model]) -> Warning {
    Warning {
        code: 14,
        message: "The following models were commented out as we could not retrieve columns for them. Please check your privileges.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(super) fn warning_enriched_with_custom_primary_key_names_in_models(affected: &[Model]) -> Warning {
    Warning {
        code: 18,
        message: "These models were enriched with custom compound id names taken from the previous Prisma schema."
            .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(super) fn warning_relations_added_from_the_previous_data_model(affected: &[Model]) -> Warning {
    Warning {
        code: 19,
        message: "Relations were copied from the previous data model due to not using foreign keys in the database. If any of the relation columns changed in the database, the relations might not be correct anymore.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(super) fn warning_top_level_item_name_is_a_dupe(affected: &[TopLevelItem]) -> Warning {
    let has_enums = affected.iter().any(|i| matches!(i.r#type, TopLevelType::Enum));
    let has_models = affected.iter().any(|i| matches!(i.r#type, TopLevelType::Model));
    let has_views = affected.iter().any(|i| matches!(i.r#type, TopLevelType::View));

    let message = if has_models && has_enums && has_views {
        "These models, views and enums were renamed due to their names being duplicates in the Prisma Schema Language."
    } else if has_models && has_enums {
        "These models and enums were renamed due to their names being duplicates in the Prisma Schema Language."
    } else if has_models && has_views {
        "These models and views were renamed due to their names being duplicates in the Prisma Schema Language."
    } else if has_enums && has_views {
        "These enums and views were renamed due to their names being duplicates in the Prisma Schema Language."
    } else if has_models {
        "These models were renamed due to their names being duplicates in the Prisma Schema Language."
    } else if has_views {
        "These views were renamed due to their names being duplicates in the Prisma Schema Language."
    } else {
        "These enums were renamed due to their names being duplicates in the Prisma Schema Language."
    };

    Warning {
        code: 20,
        message: message.into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(super) fn warning_unsupported_types_in_views(affected: &[ViewAndFieldAndType]) -> Warning {
    Warning {
        code: 21,
        message: "These fields are not supported by the Prisma Client, because Prisma currently does not support their types.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(super) fn warning_enriched_with_map_on_field_in_views(affected: &[ViewAndField]) -> Warning {
    Warning {
        code: 22,
        message: "These fields were enriched with `@map` information taken from the previous Prisma schema.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(super) fn warning_enriched_with_map_on_view(affected: &[View]) -> Warning {
    Warning {
        code: 23,
        message: "These views were enriched with `@@map` information taken from the previous Prisma schema.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(super) fn warning_views_without_identifier(affected: &[View]) -> Warning {
    Warning {
        code: 24,
        message: "The following views were ignored as they do not have a valid unique identifier or id. This is currently not supported by the Prisma Client.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(super) fn warning_enriched_with_custom_primary_key_names_in_views(affected: &[View]) -> Warning {
    Warning {
        code: 25,
        message: "These views were enriched with custom compound id names taken from the previous Prisma schema."
            .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(super) fn warning_fields_with_empty_names_in_views(affected: &[ViewAndField]) -> Warning {
    Warning {
        code: 26,
        message: "These fields were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` attribute."
            .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(super) fn partition_tables_found(affected: &[Model]) -> Warning {
    let message = "These tables are partition tables, which are not yet fully supported.";

    Warning {
        code: 27,
        message: message.into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(super) fn inherited_tables_found(affected: &[Model]) -> Warning {
    let message = "These tables are inherited tables, which are not yet fully supported.";

    Warning {
        code: 28,
        message: message.into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}
