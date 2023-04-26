//! Warnings generator for Introspection

use std::fmt;

/// Collections used for warning generation. These should be preferred
/// over directly creating warnings from the code, to prevent spamming
/// the user.
#[derive(Debug, Default, PartialEq)]
pub struct Warnings {
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
    /// The field in the composite type has a type we do not currently support in Prisma.
    pub unsupported_types_in_type: Vec<TypeAndFieldAndType>,
    /// The name of the model is taken from a previous data model.
    pub remapped_models: Vec<Model>,
    /// The name of the view is taken from a previous data model.
    pub remapped_views: Vec<View>,
    /// The name of the enum variant is taken from a previous data model.
    pub remapped_values: Vec<EnumAndValue>,
    /// The name of the enum is taken from a previous data model.
    pub remapped_enums: Vec<Enum>,
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
        Self::default()
    }

    /// True if we have no warnings
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }

    /// True, if the datamodel has Prisma 1 style defaults
    pub fn uses_prisma_1_defaults(&self) -> bool {
        !self.prisma_1_uuid_defaults.is_empty() || !self.prisma_1_cuid_defaults.is_empty()
    }
}

impl fmt::Display for Warnings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("*** WARNING ***\n")?;

        fn render_warnings<T>(msg: &str, items: &[T], f: &mut fmt::Formatter<'_>) -> fmt::Result
        where
            T: fmt::Display,
        {
            if !items.is_empty() {
                writeln!(f)?;
                f.write_str(msg)?;
                writeln!(f)?;

                for item in items {
                    writeln!(f, "  - {item}")?;
                }
            }

            Ok(())
        }

        render_warnings(
            "These id fields had a `@default(uuid())` added because we believe the schema was created by Prisma 1:",
            &self.prisma_1_uuid_defaults,
            f,
        )?;

        render_warnings(
            "These id fields had a `@default(cuid())` added because we believe the schema was created by Prisma 1:",
            &self.prisma_1_cuid_defaults,
            f,
        )?;

        render_warnings(
            "These fields were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` attribute:",
            &self.fields_with_empty_names_in_model,
            f
        )?;

        render_warnings(
            "These fields were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` attribute:",
            &self.fields_with_empty_names_in_view,
            f
        )?;

        render_warnings(
            "These fields were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` attribute:",
            &self.fields_with_empty_names_in_type,
            f
        )?;

        render_warnings(
            "These fields were enriched with `@map` information taken from the previous Prisma schema:",
            &self.remapped_fields_in_model,
            f,
        )?;

        render_warnings(
            "These fields were enriched with `@map` information taken from the previous Prisma schema:",
            &self.remapped_fields_in_view,
            f,
        )?;

        render_warnings(
            "These enum values were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` attribute:",
            &self.enum_values_with_empty_names,
            f
        )?;

        render_warnings(
            "The following models were commented out as we could not retrieve columns for them. Please check your privileges:",
            &self.models_without_columns,
            f
        )?;

        render_warnings(
            "The following models were ignored as they do not have a valid unique identifier or id. This is currently not supported by the Prisma Client:",
            &self.models_without_identifiers,
            f
        )?;

        render_warnings(
            "The following views were ignored as they do not have a valid unique identifier or id. This is currently not supported by the Prisma Client. Please refer to the documentation on defining unique identifiers in views: https://pris.ly/d/view-identifiers",
            &self.views_without_identifiers,
            f
        )?;

        render_warnings(
            "These models were enriched with custom compound id names taken from the previous Prisma schema:",
            &self.reintrospected_id_names_in_model,
            f,
        )?;

        render_warnings(
            "These views were enriched with custom compound id names taken from the previous Prisma schema:",
            &self.reintrospected_id_names_in_view,
            f,
        )?;

        render_warnings(
            "These fields are not supported by the Prisma Client, because Prisma currently does not support their types:",
            &self.unsupported_types_in_model,
            f,
        )?;

        render_warnings(
            "These fields are not supported by the Prisma Client, because Prisma currently does not support their types:",
            &self.unsupported_types_in_view,
            f,
        )?;

        render_warnings(
            "These fields are not supported by the Prisma Client, because Prisma currently does not support their types:",
            &self.unsupported_types_in_type,
            f,
        )?;

        render_warnings(
            "These models were enriched with `@@map` information taken from the previous Prisma schema:",
            &self.remapped_models,
            f,
        )?;

        render_warnings(
            "These views were enriched with `@@map` information taken from the previous Prisma schema:",
            &self.remapped_views,
            f,
        )?;

        render_warnings(
            "These enum values were enriched with `@map` information taken from the previous Prisma schema:",
            &self.remapped_values,
            f,
        )?;

        render_warnings(
            "These enums were enriched with `@@map` information taken from the previous Prisma schema:",
            &self.remapped_enums,
            f,
        )?;

        render_warnings(
            "Relations were copied from the previous data model due to not using foreign keys in the database. If any of the relation columns changed in the database, the relations might not be correct anymore:",
            &self.reintrospected_relations,
            f,
        )?;

        render_warnings(
            "These items were renamed due to their names being duplicates in the Prisma Schema Language:",
            &self.duplicate_names,
            f,
        )?;

        render_warnings(
            "These tables are partition tables, which are not yet fully supported:",
            &self.partition_tables,
            f,
        )?;

        render_warnings(
            "These tables are inherited tables, which are not yet fully supported:",
            &self.inherited_tables,
            f,
        )?;

        render_warnings(
            "These index columns are having a non-default null sort order, which is not yet fully supported. Read more: https://pris.ly/d/non-default-index-null-ordering",
            &self.non_default_index_null_sort_order,
            f,
        )?;

        render_warnings(
            "These tables contain row level security, which is not yet fully supported. Read more: https://pris.ly/d/row-level-security",
            &self.row_level_security_tables,
            f,
        )?;

        render_warnings(
            "These constraints are not supported by the Prisma Client, because Prisma currently does not fully support check constraints. Read more: https://pris.ly/d/postgres-check-constraints",
            &self.check_constraints,
            f,
        )?;

        render_warnings(
            "These constraints are not supported by the Prisma Client, because Prisma currently does not fully support exclusion constraints. Read more: https://pris.ly/d/postgres-exclusion-constraints",
            &self.exclusion_constraints,
            f,
        )?;

        render_warnings(
            "These models are using a row level TTL setting defined in the database, which is not yet fully supported. Read more: https://pris.ly/d/row-level-ttl",
            &self.row_level_ttl,
            f,
        )?;

        render_warnings(
            "These primary key, foreign key or unique constraints are using non-default deferring in the database, which is not yet fully supported. Read more: https://pris.ly/d/constraint-deferring",
            &self.non_default_deferring,
            f,
        )?;

        render_warnings(
            "These objects have comments defined in the database, which is not yet fully supported. Read more: https://pris.ly/d/database-comments",
            &self.objects_with_comments,
            f,
        )?;

        render_warnings(
            "The following fields point to nested objects without any data:",
            &self.model_fields_pointing_to_an_empty_type,
            f,
        )?;

        render_warnings(
            "The following fields point to nested objects without any data:",
            &self.type_fields_pointing_to_an_empty_type,
            f,
        )?;

        render_warnings(
            "Could not determine the types for the following fields:",
            &self.model_fields_with_unknown_type,
            f,
        )?;

        render_warnings(
            "Could not determine the types for the following fields:",
            &self.type_fields_with_unknown_type,
            f,
        )?;

        render_warnings(
            "The following fields had data stored in multiple types. Either use Json or normalize data to the wanted type:",
            &self.undecided_types_in_models,
            f,
        )?;

        render_warnings(
            "The following fields had data stored in multiple types. Either use Json or normalize data to the wanted type:",
            &self.undecided_types_in_types,
            f,
        )?;

        Ok(())
    }
}

/// A model that triggered a warning.
#[derive(PartialEq, Debug, Clone)]
pub struct Model {
    /// The name of the model
    pub model: String,
}

impl fmt::Display for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, r#""{}""#, self.model)
    }
}

/// A view that triggered a warning.
#[derive(PartialEq, Debug, Clone)]
pub struct View {
    /// The name of the view
    pub view: String,
}

impl fmt::Display for View {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, r#""{}""#, self.view)
    }
}

/// An enum that triggered a warning.
#[derive(PartialEq, Debug, Clone)]
pub struct Enum {
    /// The name of the enum
    pub r#enum: String,
}

impl fmt::Display for Enum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, r#""{}""#, self.r#enum)
    }
}

/// A field in a model that triggered a warning.
#[derive(PartialEq, Debug, Clone)]
pub struct ModelAndField {
    /// The name of the model
    pub model: String,
    /// The name of the field
    pub field: String,
}

impl fmt::Display for ModelAndField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, r#"Model: "{}", field: "{}""#, self.model, self.field)
    }
}

/// A field in a type that triggered a warning.
#[derive(PartialEq, Debug, Clone)]
pub struct TypeAndField {
    /// The name of the model
    pub composite_type: String,
    /// The name of the field
    pub field: String,
}

impl fmt::Display for TypeAndField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"Composite type: "{}", field: "{}""#,
            self.composite_type, self.field
        )
    }
}

/// A field in a view that triggered a warning.
#[derive(PartialEq, Debug, Clone)]
pub struct ViewAndField {
    /// The name of the view
    pub view: String,
    /// The name of the field
    pub field: String,
}

impl fmt::Display for ViewAndField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, r#"View: "{}", field: "{}""#, self.view, self.field)
    }
}

/// An index in a model that triggered a warning.
#[derive(PartialEq, Debug, Clone)]
pub struct ModelAndIndex {
    /// The name of the model
    pub model: String,
    /// The name of the index
    pub index_db_name: String,
}

impl fmt::Display for ModelAndIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, r#"Model: "{}", index: "{}""#, self.model, self.index_db_name)
    }
}

/// A constraint in a model that triggered a warning.
#[derive(PartialEq, Debug, Clone)]
pub struct ModelAndConstraint {
    /// The name of the model
    pub model: String,
    /// The name of the constraint
    pub constraint: String,
}

impl fmt::Display for ModelAndConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, r#"Model: "{}", constraint: "{}""#, self.model, self.constraint)
    }
}

/// A field type in a model that triggered a warning.
#[derive(PartialEq, Debug)]
pub struct ModelAndFieldAndType {
    /// The name of the model
    pub model: String,
    /// The name of the field
    pub field: String,
    /// The name of the type
    pub r#type: String,
}

impl fmt::Display for ModelAndFieldAndType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"Model: "{}", field: "{}", type: "{}""#,
            self.model, self.field, self.r#type
        )
    }
}

/// A field type in a view that triggered a warning.
#[derive(PartialEq, Debug)]
pub struct ViewAndFieldAndType {
    /// The name of the view
    pub view: String,
    /// The name of the field
    pub field: String,
    /// The name of the type
    pub r#type: String,
}

impl fmt::Display for ViewAndFieldAndType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"View: "{}", field: "{}", type: "{}""#,
            self.view, self.field, self.r#type
        )
    }
}

/// A field type in a type that triggered a warning.
#[derive(PartialEq, Debug)]
pub struct TypeAndFieldAndType {
    /// The name of the type
    pub composite_type: String,
    /// The name of the field
    pub field: String,
    /// The name of the type
    pub r#type: String,
}

impl fmt::Display for TypeAndFieldAndType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"Composite type: "{}", field: "{}", type: "{}""#,
            self.composite_type, self.field, self.r#type
        )
    }
}

/// An enum value that triggered a warning.
#[derive(PartialEq, Debug, Clone)]
pub struct EnumAndValue {
    /// The name of the enum
    pub r#enum: String,
    /// The enum value
    pub value: String,
}

impl fmt::Display for EnumAndValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, r#"Enum: "{}", value: "{}""#, self.r#enum, self.value)
    }
}

/// An top level type that triggered a warning.
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum TopLevelType {
    /// A model.
    Model,
    /// An enum.
    Enum,
    /// A view.
    View,
}

impl AsRef<str> for TopLevelType {
    fn as_ref(&self) -> &str {
        match self {
            TopLevelType::Model => "model",
            TopLevelType::Enum => "enum",
            TopLevelType::View => "view",
        }
    }
}

impl fmt::Display for TopLevelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

/// An top level item that triggered a warning.
#[derive(PartialEq, Debug, Clone)]
pub struct TopLevelItem {
    /// The name of the top-level type
    pub r#type: TopLevelType,
    /// The name of the object
    pub name: String,
}

impl fmt::Display for TopLevelItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, r#"Type: "{}", name: "{}""#, self.r#type, self.name)
    }
}

/// An object in the PSL.
#[derive(PartialEq, Debug, Clone)]
pub struct Object {
    /// The type of the object.
    pub r#type: &'static str,
    /// The name of the object.
    pub name: String,
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, r#"Type: "{}", name: "{}""#, self.r#type, self.name)
    }
}

/// An indexed column that triggered a warning.
#[derive(PartialEq, Debug, Clone)]
pub struct IndexedColumn {
    /// The name of the index
    pub index_name: String,
    /// The name of the column
    pub column_name: String,
}

impl fmt::Display for IndexedColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, r#"Index: "{}", column: "{}""#, self.index_name, self.column_name)
    }
}
