//! Warnings generator for Introspection

mod warning;

use serde::Serialize;
pub use warning::Warning;

/// Collections used for warning generation. These should be preferred
/// over directly creating warnings from the code, to prevent spamming
/// the user.
#[derive(Debug, Default)]
pub struct Warnings {
    /// Store final warnings to this vector.
    warnings: Vec<Warning>,
    /// Fields that are using Prisma 1 UUID defaults.
    pub prisma_1_uuid_defaults: Vec<ModelAndField>,
    /// Fields that are using Prisma 1 CUID defaults.
    pub prisma_1_cuid_defaults: Vec<ModelAndField>,
    /// Fields having an empty name.
    pub fields_with_empty_names_in_model: Vec<ModelAndField>,
    /// Fields having an empty name.
    pub fields_with_empty_names_in_view: Vec<ViewAndField>,
    /// Fields having an empty name.
    pub fields_with_empty_names_in_type: Vec<TypeAndField>,
    /// Field names in models we remapped during introspection.
    pub remapped_fields_in_model: Vec<ModelAndField>,
    /// Field names in views we remapped during introspection.
    pub remapped_fields_in_view: Vec<ViewAndField>,
    /// Enum values that are empty strings.
    pub enum_values_with_empty_names: Vec<EnumAndValue>,
    /// Models that have no fields.
    pub models_without_columns: Vec<Model>,
    /// Models missing a id or unique constraint.
    pub models_without_identifiers: Vec<Model>,
    /// Views missing a id or unique constraint.
    pub views_without_identifiers: Vec<View>,
    /// If the id attribute has a name taken from a previous model.
    pub reintrospected_id_names_in_model: Vec<Model>,
    /// If the id attribute has a name taken from a previous view.
    pub reintrospected_id_names_in_view: Vec<View>,
    /// The field in model has a type we do not currently support in Prisma.
    pub unsupported_types_in_model: Vec<ModelAndFieldAndType>,
    /// The field in view has a type we do not currently support in Prisma.
    pub unsupported_types_in_view: Vec<ViewAndFieldAndType>,
    /// The field in view has a type we do not currently support in Prisma.
    pub unsupported_types_in_type: Vec<TypeAndFieldAndType>,
    /// The name of the model is taken from a previous data model.
    pub remapped_models: Vec<Model>,
    /// The name of the model is taken from a previous data model.
    pub remapped_views: Vec<View>,
    /// The name of the enum variant is taken from a previous data model.
    pub remapped_values: Vec<EnumAndValue>,
    /// The relation is copied from a previous data model, only if
    /// `relationMode` is `prisma`.
    pub reintrospected_relations: Vec<Model>,
    /// The name of these models or enums was a dupe in the PSL.
    pub duplicate_names: Vec<TopLevelItem>,
    /// Warn about using partition tables, which only have introspection support.
    pub partition_tables: Vec<Model>,
    /// Warn about using inherited tables, which only have introspection support.
    pub inherited_tables: Vec<Model>,
    /// Warn about non-default NULLS FIRST/NULLS LAST in indices.
    pub non_default_index_null_sort_order: Vec<IndexedColumn>,
    /// Warn about using row level security, which is currently unsupported.
    pub row_level_security_tables: Vec<Model>,
    /// Warn about check constraints.
    pub check_constraints: Vec<ModelAndConstraint>,
    /// Warn about exclusion constraints.
    pub exclusion_constraints: Vec<ModelAndConstraint>,
    /// Warn about row level TTL
    pub row_level_ttl: Vec<Model>,
    /// Warn about non-default unique deferring setup
    pub non_default_deferring: Vec<ModelAndConstraint>,
    /// Warn about comments
    pub objects_with_comments: Vec<Object>,
    /// Warn about fields which point to an empty type.
    pub model_fields_pointing_to_an_empty_type: Vec<ModelAndField>,
    /// Warn about compositefields which point to an empty type.
    pub type_fields_pointing_to_an_empty_type: Vec<TypeAndField>,
    /// Warn about unknown types in a model.
    pub model_fields_with_unknown_type: Vec<ModelAndField>,
    /// Warn about unknown types in a composite type.
    pub type_fields_with_unknown_type: Vec<TypeAndField>,
    /// Warn about undecided types in a model.
    pub undecided_types_in_models: Vec<ModelAndFieldAndType>,
    /// Warn about undecided types in a composite type.
    pub undecided_types_in_types: Vec<TypeAndFieldAndType>,
}

impl Warnings {
    /// Generate a new empty warnings structure.
    pub fn new() -> Self {
        Self {
            warnings: Vec::new(),
            ..Default::default()
        }
    }

    /// Push a warning to the collection.
    pub fn push(&mut self, warning: Warning) {
        self.warnings.push(warning);
    }

    /// Generate warnings from all indicators. Must be called after
    /// introspection.
    pub fn finalize(mut self) -> Vec<Warning> {
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
            &self.unsupported_types_in_type,
            warning_unsupported_types_in_types,
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
            &self.fields_with_empty_names_in_type,
            warning_fields_with_empty_names_in_types,
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

        maybe_warn(
            &self.row_level_security_tables,
            row_level_security_tables_found,
            &mut self.warnings,
        );

        maybe_warn(
            &self.non_default_index_null_sort_order,
            non_default_index_null_sort_order,
            &mut self.warnings,
        );

        maybe_warn(&self.check_constraints, check_constraints_found, &mut self.warnings);

        maybe_warn(
            &self.exclusion_constraints,
            exclusion_constraints_found,
            &mut self.warnings,
        );

        maybe_warn(&self.row_level_ttl, row_level_ttl_in_tables, &mut self.warnings);
        maybe_warn(&self.non_default_deferring, non_default_deferring, &mut self.warnings);
        maybe_warn(&self.objects_with_comments, commented_objects, &mut self.warnings);

        maybe_warn(
            &self.model_fields_pointing_to_an_empty_type,
            model_fields_pointing_an_empty_type,
            &mut self.warnings,
        );

        maybe_warn(
            &self.type_fields_pointing_to_an_empty_type,
            type_fields_pointing_an_empty_type,
            &mut self.warnings,
        );

        maybe_warn(
            &self.model_fields_with_unknown_type,
            warning_unknown_types_in_models,
            &mut self.warnings,
        );

        maybe_warn(
            &self.type_fields_with_unknown_type,
            warning_unknown_types_in_types,
            &mut self.warnings,
        );

        maybe_warn(
            &self.undecided_types_in_models,
            warning_undecided_types_in_models,
            &mut self.warnings,
        );

        maybe_warn(
            &self.undecided_types_in_types,
            warning_undecided_types_in_types,
            &mut self.warnings,
        );

        self.warnings
    }
}

/// A model that triggered a warning.
#[derive(Serialize, Debug, Clone)]
pub struct Model {
    /// The name of the model
    pub model: String,
}

/// A view that triggered a warning.
#[derive(Serialize, Debug, Clone)]
pub struct View {
    /// The name of the view
    pub view: String,
}

/// An enum that triggered a warning.
#[derive(Serialize, Debug, Clone)]
pub struct Enum {
    /// The name of the enum
    pub enm: String,
}

impl Enum {
    /// Create a new enum with given name.
    pub fn new(name: &str) -> Self {
        Enum { enm: name.to_owned() }
    }
}

/// A field in a model that triggered a warning.
#[derive(Serialize, Debug, Clone)]
pub struct ModelAndField {
    /// The name of the model
    pub model: String,
    /// The name of the field
    pub field: String,
}

/// A field in a type that triggered a warning.
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TypeAndField {
    /// The name of the model
    pub composite_type: String,
    /// The name of the field
    pub field: String,
}

/// A field in a view that triggered a warning.
#[derive(Serialize, Debug, Clone)]
pub struct ViewAndField {
    /// The name of the view
    pub view: String,
    /// The name of the field
    pub field: String,
}

/// An index in a model that triggered a warning.
#[derive(Serialize, Debug, Clone)]
pub struct ModelAndIndex {
    /// The name of the model
    pub model: String,
    /// The name of the index
    pub index_db_name: String,
}

/// A constraint in a model that triggered a warning.
#[derive(Serialize, Debug, Clone)]
pub struct ModelAndConstraint {
    /// The name of the model
    pub model: String,
    /// The name of the constraint
    pub constraint: String,
}

/// A field type in a model that triggered a warning.
#[derive(Serialize, Debug)]
pub struct ModelAndFieldAndType {
    /// The name of the model
    pub model: String,
    /// The name of the field
    pub field: String,
    /// The name of the type
    pub tpe: String,
}

/// A field type in a view that triggered a warning.
#[derive(Serialize, Debug)]
pub struct ViewAndFieldAndType {
    /// The name of the view
    pub view: String,
    /// The name of the field
    pub field: String,
    /// The name of the type
    pub tpe: String,
}

/// A field type in a type that triggered a warning.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TypeAndFieldAndType {
    /// The name of the type
    pub composite_type: String,
    /// The name of the field
    pub field: String,
    /// The name of the type
    pub tpe: String,
}

/// An enum value that triggered a warning.
#[derive(Serialize, Debug, Clone)]
pub struct EnumAndValue {
    /// The name of the enum
    pub enm: String,
    /// The enum value
    pub value: String,
}

/// An top level type that triggered a warning.
#[derive(Serialize, Debug, Clone, Copy)]
pub enum TopLevelType {
    /// A model.
    Model,
    /// An enum.
    Enum,
    /// A view.
    View,
}

/// An top level item that triggered a warning.
#[derive(Serialize, Debug, Clone)]
pub struct TopLevelItem {
    /// The name of the top-level type
    pub r#type: TopLevelType,
    /// The name of the object
    pub name: String,
}

/// An object in the PSL.
#[derive(Serialize, Debug, Clone)]
pub struct Object {
    /// The type of the object.
    pub r#type: &'static str,
    /// The name of the object.
    pub name: String,
}

/// An indexed column that triggered a warning.
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IndexedColumn {
    /// The name of the index
    pub index_name: String,
    /// The name of the column
    pub column_name: String,
}

fn warning_models_without_identifier(affected: &[Model]) -> Warning {
    Warning {
        code: 1,
        message: "The following models were ignored as they do not have a valid unique identifier or id. This is currently not supported by the Prisma Client.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn warning_fields_with_empty_names_in_models(affected: &[ModelAndField]) -> Warning {
    Warning {
        code: 2,
        message: "These fields were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` attribute."
            .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn warning_unsupported_types_in_models(affected: &[ModelAndFieldAndType]) -> Warning {
    Warning {
        code: 3,
        message: "These fields are not supported by the Prisma Client, because Prisma currently does not support their types.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn warning_enum_values_with_empty_names(affected: &[EnumAndValue]) -> Warning {
    Warning {
        code: 4,
        message: "These enum values were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` attribute."
            .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn warning_default_cuid_warning(affected: &[ModelAndField]) -> Warning {
    Warning {
        code: 5,
        message:
            "These id fields had a `@default(cuid())` added because we believe the schema was created by Prisma 1."
                .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn warning_default_uuid_warning(affected: &[ModelAndField]) -> Warning {
    Warning {
        code: 6,
        message:
            "These id fields had a `@default(uuid())` added because we believe the schema was created by Prisma 1."
                .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn warning_enriched_with_map_on_model(affected: &[Model]) -> Warning {
    Warning {
        code: 7,
        message: "These models were enriched with `@@map` information taken from the previous Prisma schema.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn warning_enriched_with_map_on_field_in_models(affected: &[ModelAndField]) -> Warning {
    Warning {
        code: 8,
        message: "These fields were enriched with `@map` information taken from the previous Prisma schema.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

/// A model was given a `@@map` attribute during introspection.
pub fn warning_enriched_with_map_on_enum(affected: &[Enum]) -> Warning {
    Warning {
        code: 9,
        message: "These enums were enriched with `@@map` information taken from the previous Prisma schema.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn warning_enriched_with_map_on_enum_value(affected: &[EnumAndValue]) -> Warning {
    Warning {
        code: 10,
        message: "These enum values were enriched with `@map` information taken from the previous Prisma schema."
            .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

//todo maybe we can get rid of this alltogether due to @@ignore
//but maybe we should have warnings for ignored fields and models
fn warning_models_without_columns(affected: &[Model]) -> Warning {
    Warning {
        code: 14,
        message: "The following models were commented out as we could not retrieve columns for them. Please check your privileges.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn warning_enriched_with_custom_primary_key_names_in_models(affected: &[Model]) -> Warning {
    Warning {
        code: 18,
        message: "These models were enriched with custom compound id names taken from the previous Prisma schema."
            .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn warning_relations_added_from_the_previous_data_model(affected: &[Model]) -> Warning {
    Warning {
        code: 19,
        message: "Relations were copied from the previous data model due to not using foreign keys in the database. If any of the relation columns changed in the database, the relations might not be correct anymore.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn warning_top_level_item_name_is_a_dupe(affected: &[TopLevelItem]) -> Warning {
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

fn warning_unsupported_types_in_views(affected: &[ViewAndFieldAndType]) -> Warning {
    Warning {
        code: 21,
        message: "These fields are not supported by the Prisma Client, because Prisma currently does not support their types.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn warning_enriched_with_map_on_field_in_views(affected: &[ViewAndField]) -> Warning {
    Warning {
        code: 22,
        message: "These fields were enriched with `@map` information taken from the previous Prisma schema.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn warning_enriched_with_map_on_view(affected: &[View]) -> Warning {
    Warning {
        code: 23,
        message: "These views were enriched with `@@map` information taken from the previous Prisma schema.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn warning_views_without_identifier(affected: &[View]) -> Warning {
    Warning {
        code: 24,
        message: "The following views were ignored as they do not have a valid unique identifier or id. This is currently not supported by the Prisma Client. Please refer to the documentation on defining unique identifiers in views: https://pris.ly/d/view-identifiers".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn warning_enriched_with_custom_primary_key_names_in_views(affected: &[View]) -> Warning {
    Warning {
        code: 25,
        message: "These views were enriched with custom compound id names taken from the previous Prisma schema."
            .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn warning_fields_with_empty_names_in_views(affected: &[ViewAndField]) -> Warning {
    Warning {
        code: 26,
        message: "These fields were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` attribute."
            .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn partition_tables_found(affected: &[Model]) -> Warning {
    let message = "These tables are partition tables, which are not yet fully supported.";

    Warning {
        code: 27,
        message: message.into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn inherited_tables_found(affected: &[Model]) -> Warning {
    let message = "These tables are inherited tables, which are not yet fully supported.";

    Warning {
        code: 28,
        message: message.into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn non_default_index_null_sort_order(affected: &[IndexedColumn]) -> Warning {
    let message = "These index columns are having a non-default null sort order, which is not yet fully supported. Read more: https://pris.ly/d/non-default-index-null-ordering";

    Warning {
        code: 29,
        message: message.into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn row_level_security_tables_found(affected: &[Model]) -> Warning {
    let message = "These tables contain row level security, which is not yet fully supported. Read more: https://pris.ly/d/row-level-security";

    Warning {
        code: 30,
        message: message.into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn row_level_ttl_in_tables(affected: &[Model]) -> Warning {
    let message = "These models are using a row level TTL setting defined in the database, which is not yet fully supported. Read more: https://pris.ly/d/row-level-ttl";

    Warning {
        code: 31,
        message: message.into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn check_constraints_found(affected: &[ModelAndConstraint]) -> Warning {
    let message = "These constraints are not supported by the Prisma Client, because Prisma currently does not fully support check constraints. Read more: https://pris.ly/d/postgres-check-constraints";

    Warning {
        code: 33,
        message: message.into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn exclusion_constraints_found(affected: &[ModelAndConstraint]) -> Warning {
    let message = "These constraints are not supported by the Prisma Client, because Prisma currently does not fully support exclusion constraints. Read more: https://pris.ly/d/postgres-exclusion-constraints";

    Warning {
        code: 34,
        message: message.into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn non_default_deferring(affected: &[ModelAndConstraint]) -> Warning {
    let message = "These primary key, foreign key or unique constraints are using non-default deferring in the database, which is not yet fully supported. Read more: https://pris.ly/d/constraint-deferring";

    Warning {
        code: 35,
        message: message.into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn commented_objects(affected: &[Object]) -> Warning {
    let message = "These objects have comments defined in the database, which is not yet fully supported. Read more: https://pris.ly/d/database-comments";

    Warning {
        code: 36,
        message: message.into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn model_fields_pointing_an_empty_type(affected: &[ModelAndField]) -> Warning {
    let message = "The following fields point to nested objects without any data.";

    Warning {
        code: 37,
        message: message.to_string(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn type_fields_pointing_an_empty_type(affected: &[TypeAndField]) -> Warning {
    let message = "The following fields point to nested objects without any data.";

    Warning {
        code: 38,
        message: message.to_string(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn warning_unsupported_types_in_types(affected: &[TypeAndFieldAndType]) -> Warning {
    Warning {
        code: 39,
        message: "These fields are not supported by the Prisma Client, because Prisma currently does not support their types.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn warning_unknown_types_in_models(affected: &[ModelAndField]) -> Warning {
    Warning {
        code: 40,
        message: "Could not determine the types for the following fields.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn warning_unknown_types_in_types(affected: &[TypeAndField]) -> Warning {
    Warning {
        code: 41,
        message: "Could not determine the types for the following fields.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn warning_undecided_types_in_models(affected: &[ModelAndFieldAndType]) -> Warning {
    Warning {
        code: 42,
        message: "The following fields had data stored in multiple types. Either use Json or normalize data to the wanted type.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn warning_undecided_types_in_types(affected: &[TypeAndFieldAndType]) -> Warning {
    Warning {
        code: 43,
        message: "The following fields had data stored in multiple types. Either use Json or normalize data to the wanted type.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

fn warning_fields_with_empty_names_in_types(affected: &[TypeAndField]) -> Warning {
    Warning {
        code: 44,
        message: "These fields were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` attribute."
            .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}
