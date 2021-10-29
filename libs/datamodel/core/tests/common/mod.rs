use datamodel::{
    diagnostics::*,
    dml::{self, ScalarType},
    Configuration, Datamodel, IndexDefinition, Model, NativeTypeInstance, PrimaryKeyDefinition, StringFromEnvVar,
};
use pretty_assertions::assert_eq;

pub(crate) use expect_test::expect;
pub(crate) use indoc::formatdoc;
pub(crate) use indoc::indoc;

pub(crate) trait DatasourceAsserts {
    fn assert_name(&self, name: &str) -> &Self;
    fn assert_url(&self, url: StringFromEnvVar) -> &Self;
}

pub(crate) trait FieldAsserts {
    fn assert_arity(&self, arity: &dml::FieldArity) -> &Self;
    fn assert_with_documentation(&self, t: &str) -> &Self;
    fn assert_is_generated(&self, b: bool) -> &Self;
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

pub(crate) trait EnumAsserts {
    fn assert_has_value(&self, t: &str) -> &dml::EnumValue;

    fn assert_with_documentation(&self, t: &str) -> &Self;
}

pub(crate) trait EnumValueAsserts {
    fn assert_with_documentation(&self, t: &str) -> &Self;
}

pub(crate) trait DatamodelAsserts {
    fn assert_has_model(&self, t: &str) -> &dml::Model;
    fn assert_has_enum(&self, t: &str) -> &dml::Enum;
}

pub(crate) trait ErrorAsserts {
    fn assert_is(&self, error: DatamodelError) -> &Self;
    fn assert_are(&self, error: &[DatamodelError]) -> &Self;
    fn assert_is_message(&self, msg: &str) -> &Self;
    fn assert_is_at(&self, index: usize, error: DatamodelError) -> &Self;
    fn assert_length(&self, length: usize) -> &Self;
    fn assert_is_message_at(&self, index: usize, msg: &str) -> &Self;
}

pub(crate) trait WarningAsserts {
    fn assert_is(&self, warning: DatamodelWarning) -> &Self;
}

impl DatasourceAsserts for datamodel::Datasource {
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

    fn assert_is_generated(&self, b: bool) -> &Self {
        assert_eq!(self.is_generated, b);
        self
    }
}

impl ScalarFieldAsserts for dml::ScalarField {
    fn assert_base_type(&self, t: &ScalarType) -> &Self {
        if let dml::FieldType::Scalar(base_type, _, None) = &self.field_type {
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
        if let dml::FieldType::Scalar(_, _, Some(t)) = &self.field_type {
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

impl FieldAsserts for dml::RelationField {
    fn assert_arity(&self, arity: &dml::FieldArity) -> &Self {
        assert_eq!(self.arity, *arity);
        self
    }

    fn assert_with_documentation(&self, t: &str) -> &Self {
        assert_eq!(self.documentation, Some(t.to_owned()));
        self
    }

    fn assert_is_generated(&self, b: bool) -> &Self {
        assert_eq!(self.is_generated, b);
        self
    }
}

impl RelationFieldAsserts for dml::RelationField {
    fn assert_relation_name(&self, t: &str) -> &Self {
        assert_eq!(self.relation_info.name, t.to_owned());
        self
    }

    fn assert_relation_to(&self, t: &str) -> &Self {
        assert_eq!(self.relation_info.to, t);
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
        self.find_model(&t.to_owned())
            .unwrap_or_else(|| panic!("Model {} not found", t))
    }
    fn assert_has_enum(&self, t: &str) -> &dml::Enum {
        self.find_enum(&t.to_owned())
            .unwrap_or_else(|| panic!("Enum {} not found", t))
    }
}

impl ModelAsserts for dml::Model {
    fn assert_has_scalar_field(&self, t: &str) -> &dml::ScalarField {
        self.find_scalar_field(&t.to_owned())
            .unwrap_or_else(|| panic!("Field {} not found", t))
    }

    fn assert_has_relation_field(&self, t: &str) -> &dml::RelationField {
        self.find_relation_field(&t.to_owned())
            .unwrap_or_else(|| panic!("Field {} not found", t))
    }

    fn assert_with_db_name(&self, t: &str) -> &Self {
        assert_eq!(self.database_name, Some(t.to_owned()));
        self
    }

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
        assert_eq!(self.primary_key.as_ref().unwrap().fields, fields);
        self
    }

    fn assert_has_named_pk(&self, name: &str) -> &Self {
        assert_eq!(self.primary_key.as_ref().unwrap().db_name, Some(name.to_string()));
        self
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

impl ErrorAsserts for Diagnostics {
    fn assert_is(&self, error: DatamodelError) -> &Self {
        assert_eq!(
            self.errors().len(),
            1,
            "Expected exactly one validation error. Errors are: {:?}",
            &self
        );
        assert_eq!(self.errors()[0], error);
        self
    }

    fn assert_are(&self, errors: &[DatamodelError]) -> &Self {
        assert_eq!(self.errors(), errors);
        self
    }

    fn assert_is_message(&self, msg: &str) -> &Self {
        assert_eq!(
            self.errors().len(),
            1,
            "Expected exactly one validation error. Errors are: {:?}",
            &self
        );
        assert_eq!(self.errors()[0].description(), msg);
        self
    }

    fn assert_is_at(&self, index: usize, error: DatamodelError) -> &Self {
        assert_eq!(self.errors()[index], error);
        self
    }

    fn assert_length(&self, length: usize) -> &Self {
        assert_eq!(
            self.errors().len(),
            length,
            "Expected exactly {} validation errors, but got {}. The errors were {:?}",
            length,
            self.errors().len(),
            &self.errors(),
        );
        self
    }

    fn assert_is_message_at(&self, index: usize, msg: &str) -> &Self {
        assert_eq!(self.errors()[index].description(), msg);
        self
    }
}

pub(crate) fn parse(datamodel_string: &str) -> Datamodel {
    match datamodel::parse_datamodel(datamodel_string) {
        Ok(s) => s.subject,
        Err(errs) => {
            panic!(
                "Datamodel parsing failed\n\n{}",
                errs.to_pretty_string("", datamodel_string)
            )
        }
    }
}

pub(crate) fn parse_configuration(datamodel_string: &str) -> Configuration {
    match datamodel::parse_configuration(datamodel_string) {
        Ok(c) => c.subject,
        Err(errs) => {
            panic!(
                "Configuration parsing failed\n\n{}",
                errs.to_pretty_string("", datamodel_string)
            )
        }
    }
}

pub(crate) fn parse_and_render_error(schema: &str) -> String {
    parse_error(schema).to_pretty_string("schema.prisma", schema)
}

pub(crate) fn parse_error(datamodel_string: &str) -> Diagnostics {
    match datamodel::parse_datamodel(datamodel_string) {
        Ok(_) => panic!("Expected an error when parsing schema."),
        Err(errs) => errs,
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

pub(crate) const MSSQL_SOURCE: &str = r#"
    datasource db {
        provider = "sqlserver"
        url      = "sqlserver://localhost:1433"
    }
"#;
