pub use ::indoc::{formatdoc, indoc};
pub use expect_test::expect;
pub use psl::{dml, dml::*};

use psl::{diagnostics::*, Configuration, StringFromEnvVar};

pub(crate) fn reformat(input: &str) -> String {
    psl::reformat(input, 2).unwrap_or_else(|| input.to_owned())
}

pub(crate) trait DatasourceAsserts {
    fn assert_name(&self, name: &str) -> &Self;
    fn assert_url(&self, url: StringFromEnvVar) -> &Self;
}

pub(crate) trait FieldAsserts {
    fn assert_arity(&self, arity: &dml::FieldArity) -> &Self;
    fn assert_with_documentation(&self, t: &str) -> &Self;
}

pub(crate) trait ScalarFieldAsserts {
    fn assert_base_type(&self, t: &ScalarType) -> &Self;
    fn assert_unsupported_type(&self, t: &str) -> &Self;
    fn assert_enum_type(&self, en: &str) -> &Self;
    fn assert_native_type(&self) -> &NativeTypeInstance;
    fn assert_with_db_name(&self, t: &str) -> &Self;
    fn assert_default_value(&self, t: dml::DefaultValue) -> &Self;
    fn assert_is_id(&self, model: &Model) -> &Self;
    fn assert_is_updated_at(&self, b: bool) -> &Self;
    fn assert_ignored(&self, state: bool) -> &Self;
}

pub(crate) trait CompositeTypeFieldAsserts {
    fn assert_base_type(&self, t: &ScalarType) -> &Self;
    fn assert_default_value(&self, t: dml::DefaultValue) -> &Self;
    fn assert_enum_type(&self, en: &str) -> &Self;
}

pub(crate) trait RelationFieldAsserts {
    fn assert_relation_name(&self, t: &str) -> &Self;
    fn assert_relation_to(&self, t: &str) -> &Self;
    fn assert_relation_delete_strategy(&self, t: dml::ReferentialAction) -> &Self;
    fn assert_relation_update_strategy(&self, t: dml::ReferentialAction) -> &Self;
    fn assert_relation_referenced_fields(&self, t: &[&str]) -> &Self;
    fn assert_relation_base_fields(&self, t: &[&str]) -> &Self;
    fn assert_ignored(&self, state: bool) -> &Self;
    fn assert_relation_fk_name(&self, name: Option<String>) -> &Self;
}

pub(crate) trait ModelAsserts {
    fn assert_field_count(&self, count: usize) -> &Self;
    fn assert_has_scalar_field(&self, t: &str) -> &dml::ScalarField;
    fn assert_has_relation_field(&self, t: &str) -> &dml::RelationField;
    fn assert_with_db_name(&self, t: &str) -> &Self;
    fn assert_with_documentation(&self, t: &str) -> &Self;
    fn assert_has_index(&self, def: IndexDefinition) -> &Self;
    fn assert_has_pk(&self, pk: PrimaryKeyDefinition) -> &Self;
    fn assert_has_named_pk(&self, name: &str) -> &Self;
    fn assert_has_id_fields(&self, fields: &[&str]) -> &Self;
    fn assert_ignored(&self, state: bool) -> &Self;
}

pub(crate) trait CompositeTypeAsserts {
    fn assert_field_count(&self, count: usize) -> &Self;
    fn assert_has_scalar_field(&self, t: &str) -> &dml::CompositeTypeField;
    fn assert_has_enum_field(&self, t: &str) -> &dml::CompositeTypeField;
    fn assert_has_composite_type_field(&self, t: &str) -> &dml::CompositeTypeField;
    fn assert_has_unsupported_field(&self, t: &str) -> &dml::CompositeTypeField;
}

pub(crate) trait EnumAsserts {
    fn assert_has_value(&self, t: &str) -> &dml::EnumValue;

    fn assert_with_documentation(&self, t: &str) -> &Self;
}

pub(crate) trait EnumValueAsserts {
    fn assert_with_documentation(&self, t: &str) -> &Self;
}

pub(crate) trait DatamodelAsserts {
    fn assert_has_model(&self, t: &str) -> &dml::Model;
    fn assert_has_composite_type(&self, t: &str) -> &dml::CompositeType;
    fn assert_has_enum(&self, t: &str) -> &dml::Enum;
}

pub(crate) trait WarningAsserts {
    fn assert_is(&self, warning: DatamodelWarning) -> &Self;
}

impl DatasourceAsserts for psl::Datasource {
    fn assert_name(&self, name: &str) -> &Self {
        assert_eq!(&self.name, name);
        self
    }

    fn assert_url(&self, url: StringFromEnvVar) -> &Self {
        assert_eq!(self.url, url);
        self
    }
}

impl FieldAsserts for dml::ScalarField {
    fn assert_arity(&self, arity: &dml::FieldArity) -> &Self {
        assert_eq!(self.arity, *arity);
        self
    }

    fn assert_with_documentation(&self, t: &str) -> &Self {
        assert_eq!(self.documentation, Some(t.to_owned()));
        self
    }
}

impl ScalarFieldAsserts for dml::ScalarField {
    fn assert_base_type(&self, t: &ScalarType) -> &Self {
        if let dml::FieldType::Scalar(base_type, None) = &self.field_type {
            assert_eq!(base_type, t);
        } else {
            panic!("Scalar expected, but found {:?}", self.field_type);
        }
        self
    }

    fn assert_unsupported_type(&self, t: &str) -> &Self {
        if let dml::FieldType::Unsupported(description) = &self.field_type {
            assert_eq!(description, t);
        } else {
            panic!("Unsupported expected, but found {:?}", self.field_type);
        }
        self
    }

    fn assert_enum_type(&self, en: &str) -> &Self {
        if let dml::FieldType::Enum(enum_type) = &self.field_type {
            assert_eq!(enum_type, en);
        } else {
            panic!("Enum expected, but found {:?}", self.field_type);
        }
        self
    }

    fn assert_native_type(&self) -> &NativeTypeInstance {
        if let dml::FieldType::Scalar(_, Some(t)) = &self.field_type {
            t
        } else {
            panic!("Native Type expected, but found {:?}", self.field_type);
        }
    }

    fn assert_with_db_name(&self, t: &str) -> &Self {
        assert_eq!(self.database_name, Some(t.to_owned()));
        self
    }

    fn assert_default_value(&self, t: dml::DefaultValue) -> &Self {
        assert_eq!(self.default_value, Some(t));
        self
    }

    fn assert_is_id(&self, model: &Model) -> &Self {
        assert!(model.field_is_primary(&self.name));
        self
    }

    fn assert_is_updated_at(&self, b: bool) -> &Self {
        assert_eq!(self.is_updated_at, b);
        self
    }

    fn assert_ignored(&self, state: bool) -> &Self {
        assert_eq!(self.is_ignored, state);
        self
    }
}

impl CompositeTypeFieldAsserts for dml::CompositeTypeField {
    fn assert_base_type(&self, t: &ScalarType) -> &Self {
        if let Some((base_type, _)) = self.r#type.as_scalar() {
            assert_eq!(base_type, t);
        } else {
            panic!("Scalar expected, but found {:?}", self.r#type);
        }

        self
    }

    fn assert_default_value(&self, t: dml::DefaultValue) -> &Self {
        assert_eq!(self.default_value, Some(t));

        self
    }

    fn assert_enum_type(&self, en: &str) -> &Self {
        if let dml::CompositeTypeFieldType::Enum(enum_type) = &self.r#type {
            assert_eq!(enum_type, en);
        } else {
            panic!("Enum expected, but found {:?}", self.r#type);
        }
        self
    }
}

impl FieldAsserts for dml::RelationField {
    fn assert_arity(&self, arity: &dml::FieldArity) -> &Self {
        assert_eq!(self.arity, *arity);
        self
    }

    fn assert_with_documentation(&self, t: &str) -> &Self {
        assert_eq!(self.documentation, Some(t.to_owned()));
        self
    }
}

impl RelationFieldAsserts for dml::RelationField {
    fn assert_relation_name(&self, t: &str) -> &Self {
        assert_eq!(self.relation_info.name, t.to_owned());
        self
    }

    fn assert_relation_to(&self, t: &str) -> &Self {
        assert_eq!(self.relation_info.referenced_model, t);
        self
    }

    fn assert_relation_delete_strategy(&self, t: dml::ReferentialAction) -> &Self {
        assert_eq!(self.relation_info.on_delete, Some(t));
        self
    }

    fn assert_relation_update_strategy(&self, t: dml::ReferentialAction) -> &Self {
        assert_eq!(self.relation_info.on_update, Some(t));
        self
    }

    fn assert_relation_referenced_fields(&self, t: &[&str]) -> &Self {
        assert_eq!(self.relation_info.references, t);
        self
    }

    fn assert_relation_base_fields(&self, t: &[&str]) -> &Self {
        assert_eq!(self.relation_info.fields, t);
        self
    }

    fn assert_ignored(&self, state: bool) -> &Self {
        assert_eq!(self.is_ignored, state);
        self
    }

    fn assert_relation_fk_name(&self, name: Option<String>) -> &Self {
        assert_eq!(self.relation_info.fk_name, name);
        self
    }
}

impl DatamodelAsserts for dml::Datamodel {
    fn assert_has_model(&self, t: &str) -> &dml::Model {
        self.find_model(t).unwrap_or_else(|| panic!("Model {} not found", t))
    }
    fn assert_has_enum(&self, t: &str) -> &dml::Enum {
        self.find_enum(t).unwrap_or_else(|| panic!("Enum {} not found", t))
    }

    fn assert_has_composite_type(&self, t: &str) -> &dml::CompositeType {
        self.find_composite_type(t)
            .unwrap_or_else(|| panic!("Composite type {} not found", t))
    }
}

impl ModelAsserts for dml::Model {
    fn assert_has_scalar_field(&self, t: &str) -> &dml::ScalarField {
        self.find_scalar_field(t)
            .unwrap_or_else(|| panic!("Field {} not found", t))
    }

    fn assert_has_relation_field(&self, t: &str) -> &dml::RelationField {
        self.find_relation_field(t)
            .unwrap_or_else(|| panic!("Field {} not found", t))
    }

    fn assert_with_db_name(&self, t: &str) -> &Self {
        assert_eq!(self.database_name, Some(t.to_owned()));
        self
    }

    #[track_caller]
    fn assert_with_documentation(&self, t: &str) -> &Self {
        assert_eq!(self.documentation, Some(t.to_owned()));
        self
    }

    fn assert_has_index(&self, def: IndexDefinition) -> &Self {
        assert!(
            self.indices.contains(&def),
            "could not find index {:?} in the indexes of this model \n {:?}",
            def,
            self.indices
        );
        self
    }

    fn assert_has_pk(&self, pk: PrimaryKeyDefinition) -> &Self {
        assert_eq!(self.primary_key, Some(pk));
        self
    }

    fn assert_ignored(&self, state: bool) -> &Self {
        assert_eq!(self.is_ignored, state);
        self
    }

    fn assert_field_count(&self, count: usize) -> &Self {
        assert_eq!(self.fields.len(), count);
        self
    }

    fn assert_has_id_fields(&self, fields: &[&str]) -> &Self {
        assert_eq!(
            self.primary_key
                .as_ref()
                .unwrap()
                .fields
                .iter()
                .map(|f| &f.name)
                .collect::<Vec<_>>(),
            fields
        );
        self
    }

    fn assert_has_named_pk(&self, name: &str) -> &Self {
        assert_eq!(self.primary_key.as_ref().unwrap().db_name, Some(name.to_string()));
        self
    }
}

impl CompositeTypeAsserts for dml::CompositeType {
    fn assert_field_count(&self, count: usize) -> &Self {
        assert_eq!(self.fields.len(), count);

        self
    }

    fn assert_has_scalar_field(&self, t: &str) -> &dml::CompositeTypeField {
        self.scalar_fields()
            .find(|field| field.name == t)
            .unwrap_or_else(|| panic!("Field {} not found", t))
    }

    fn assert_has_enum_field(&self, t: &str) -> &dml::CompositeTypeField {
        self.enum_fields()
            .find(|field| field.name == t)
            .unwrap_or_else(|| panic!("Field {} not found", t))
    }

    fn assert_has_composite_type_field(&self, t: &str) -> &dml::CompositeTypeField {
        self.composite_type_fields()
            .find(|field| field.name == t)
            .unwrap_or_else(|| panic!("Field {} not found", t))
    }

    fn assert_has_unsupported_field(&self, t: &str) -> &dml::CompositeTypeField {
        self.unsupported_fields()
            .find(|field| field.name == t)
            .unwrap_or_else(|| panic!("Field {} not found", t))
    }
}

impl EnumAsserts for dml::Enum {
    fn assert_has_value(&self, t: &str) -> &dml::EnumValue {
        self.values()
            .find(|x| x.name == t)
            .unwrap_or_else(|| panic!("Enum Value {} not found", t))
    }

    fn assert_with_documentation(&self, t: &str) -> &Self {
        assert_eq!(self.documentation, Some(t.to_owned()));
        self
    }
}

impl EnumValueAsserts for dml::EnumValue {
    fn assert_with_documentation(&self, t: &str) -> &Self {
        assert_eq!(self.documentation, Some(t.to_owned()));
        self
    }
}

impl WarningAsserts for Vec<DatamodelWarning> {
    fn assert_is(&self, warning: DatamodelWarning) -> &Self {
        assert_eq!(
            self.len(),
            1,
            "Expected exactly one validation warning. Warnings are: {:?}",
            &self
        );
        assert_eq!(self[0], warning);
        self
    }
}

pub(crate) fn parse_unwrap_err(schema: &str) -> String {
    psl::parse_schema(schema).map(drop).unwrap_err()
}

#[track_caller]
pub(crate) fn parse(datamodel_string: &str) -> Datamodel {
    let schema = psl::parse_schema(datamodel_string).unwrap();
    psl::lift(&schema)
}

pub(crate) fn parse_config(schema: &str) -> Result<Configuration, String> {
    psl::parse_configuration(schema).map_err(|err| err.to_pretty_string("schema.prisma", schema))
}

pub(crate) fn parse_configuration(datamodel_string: &str) -> Configuration {
    match psl::parse_configuration(datamodel_string) {
        Ok(c) => c,
        Err(errs) => {
            panic!(
                "Configuration parsing failed\n\n{}",
                errs.to_pretty_string("", datamodel_string)
            )
        }
    }
}

#[track_caller]
pub(crate) fn expect_error(schema: &str, expectation: &expect_test::Expect) {
    match psl::parse_schema(schema) {
        Ok(_) => panic!("Expected a validation error, but the schema is valid."),
        Err(err) => expectation.assert_eq(&err),
    }
}

pub(crate) fn parse_and_render_error(schema: &str) -> String {
    parse_unwrap_err(schema)
}

#[track_caller]
pub(crate) fn assert_valid(schema: &str) {
    match psl::parse_schema(schema) {
        Ok(_) => (),
        Err(err) => panic!("{err}"),
    }
}

pub(crate) const SQLITE_SOURCE: &str = r#"
    datasource db {
        provider = "sqlite"
        url      = "file:dev.db"
    }
"#;

pub(crate) const POSTGRES_SOURCE: &str = r#"
    datasource db {
        provider = "postgres"
        url      = "postgresql://localhost:5432"
    }
"#;

pub(crate) const MYSQL_SOURCE: &str = r#"
    datasource db {
        provider = "mysql"
        url      = "mysql://localhost:3306"
    }
"#;
