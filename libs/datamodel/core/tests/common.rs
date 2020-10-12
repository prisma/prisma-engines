extern crate datamodel;

use self::datamodel::StringFromEnvVar;
use datamodel::dml::IndexDefinition;
use datamodel::dml::NativeTypeInstance;
use datamodel::{dml, dml::ScalarType, error::*};
use pretty_assertions::assert_eq;

pub trait DatasourceAsserts {
    fn assert_name(&self, name: &str) -> &Self;
    fn assert_url(&self, url: StringFromEnvVar) -> &Self;
}

pub trait FieldAsserts {
    fn assert_arity(&self, arity: &dml::FieldArity) -> &Self;
    fn assert_with_documentation(&self, t: &str) -> &Self;
    fn assert_is_generated(&self, b: bool) -> &Self;
}

pub trait ScalarFieldAsserts {
    fn assert_base_type(&self, t: &ScalarType) -> &Self;
    fn assert_enum_type(&self, en: &str) -> &Self;
    fn assert_native_type(&self) -> &NativeTypeInstance;
    fn assert_with_db_name(&self, t: &str) -> &Self;
    fn assert_default_value(&self, t: dml::DefaultValue) -> &Self;
    fn assert_is_id(&self) -> &Self;
    fn assert_is_unique(&self, b: bool) -> &Self;
    fn assert_is_updated_at(&self, b: bool) -> &Self;
}

pub trait RelationFieldAsserts {
    fn assert_relation_name(&self, t: &str) -> &Self;
    fn assert_relation_to(&self, t: &str) -> &Self;
    fn assert_relation_delete_strategy(&self, t: dml::OnDeleteStrategy) -> &Self;
    fn assert_relation_to_fields(&self, t: &[&str]) -> &Self;
    fn assert_relation_base_fields(&self, t: &[&str]) -> &Self;
}

pub trait ModelAsserts {
    fn assert_has_scalar_field(&self, t: &str) -> &dml::ScalarField;
    fn assert_has_relation_field(&self, t: &str) -> &dml::RelationField;
    fn assert_is_embedded(&self, t: bool) -> &Self;
    fn assert_with_db_name(&self, t: &str) -> &Self;
    fn assert_with_documentation(&self, t: &str) -> &Self;
    fn assert_has_index(&self, def: IndexDefinition) -> &Self;
    fn assert_has_id_fields(&self, fields: &[&str]) -> &Self;
}

pub trait EnumAsserts {
    fn assert_has_value(&self, t: &str) -> &dml::EnumValue;

    fn assert_with_documentation(&self, t: &str) -> &Self;
}

pub trait EnumValueAsserts {
    fn assert_with_documentation(&self, t: &str) -> &Self;
}

pub trait DatamodelAsserts {
    fn assert_has_model(&self, t: &str) -> &dml::Model;
    fn assert_has_enum(&self, t: &str) -> &dml::Enum;
}

pub trait ErrorAsserts {
    fn assert_is(&self, error: DatamodelError) -> &Self;
    fn assert_is_message(&self, msg: &str) -> &Self;
    fn assert_is_at(&self, index: usize, error: DatamodelError) -> &Self;
    fn assert_length(&self, length: usize) -> &Self;
}

impl DatasourceAsserts for datamodel::Datasource {
    fn assert_name(&self, name: &str) -> &Self {
        assert_eq!(&self.name, name);
        &self
    }

    fn assert_url(&self, url: StringFromEnvVar) -> &Self {
        assert_eq!(self.url, url);
        &self
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
        if let dml::FieldType::Base(base_type, _) = &self.field_type {
            assert_eq!(base_type, t);
        } else {
            panic!("Scalar expected, but found {:?}", self.field_type);
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
        if let dml::FieldType::NativeType(_, t) = &self.field_type {
            &t
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

    fn assert_is_id(&self) -> &Self {
        assert!(self.is_id);
        self
    }

    fn assert_is_unique(&self, b: bool) -> &Self {
        assert_eq!(self.is_unique, b);
        self
    }

    fn assert_is_updated_at(&self, b: bool) -> &Self {
        assert_eq!(self.is_updated_at, b);
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

    fn assert_relation_delete_strategy(&self, t: dml::OnDeleteStrategy) -> &Self {
        assert_eq!(self.relation_info.on_delete, t);
        self
    }

    fn assert_relation_to_fields(&self, t: &[&str]) -> &Self {
        assert_eq!(self.relation_info.to_fields, t);
        self
    }

    fn assert_relation_base_fields(&self, t: &[&str]) -> &Self {
        assert_eq!(self.relation_info.fields, t);
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

    fn assert_is_embedded(&self, t: bool) -> &Self {
        assert_eq!(self.is_embedded, t);

        self
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

    fn assert_has_id_fields(&self, fields: &[&str]) -> &Self {
        assert_eq!(self.id_fields, fields);
        self
    }
}

impl EnumAsserts for dml::Enum {
    fn assert_has_value(&self, t: &str) -> &dml::EnumValue {
        self.values()
            .find(|x| *x.name == t.to_owned())
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

impl ErrorAsserts for ErrorCollection {
    fn assert_is(&self, error: DatamodelError) -> &Self {
        assert_eq!(
            self.errors.len(),
            1,
            "Expected exactly one validation error. Errors are: {:?}",
            &self
        );
        assert_eq!(self.errors[0], error);

        self
    }

    fn assert_is_message(&self, msg: &str) -> &Self {
        assert_eq!(
            self.errors.len(),
            1,
            "Expected exactly one validation error. Errors are: {:?}",
            &self
        );
        assert_eq!(self.errors[0].description(), msg);

        self
    }

    fn assert_is_at(&self, index: usize, error: DatamodelError) -> &Self {
        assert_eq!(self.errors[index], error);
        self
    }

    fn assert_length(&self, length: usize) -> &Self {
        assert_eq!(
            self.errors.len(),
            length,
            "Expected exactly {} validation errors, but got {}. The errors were {:?}",
            length,
            self.errors.len(),
            &self.errors,
        );
        self
    }
}

#[allow(dead_code)] // Not sure why the compiler thinks this is never used.
pub fn parse(datamodel_string: &str) -> datamodel::Datamodel {
    match datamodel::parse_datamodel(datamodel_string) {
        Ok(s) => s,
        Err(errs) => {
            for err in errs.to_iter() {
                err.pretty_print(&mut std::io::stderr().lock(), "", datamodel_string)
                    .unwrap();
            }

            panic!("Datamodel parsing failed. Please see error above.")
        }
    }
}

#[allow(dead_code)] // Not sure why the compiler thinks this is never used.
pub fn parse_error(datamodel_string: &str) -> ErrorCollection {
    match datamodel::parse_datamodel(datamodel_string) {
        Ok(_) => panic!("Expected an error when parsing schema."),
        Err(errs) => errs,
    }
}

pub fn parse_error_and_ignore_datasource_urls(datamodel_string: &str) -> ErrorCollection {
    match datamodel::parse_datamodel_and_ignore_datasource_urls(datamodel_string) {
        Ok(_) => panic!("Expected an error when parsing schema."),
        Err(errs) => errs,
    }
}

pub const SQLITE_SOURCE: &str = r#"
    datasource db {
        provider = "sqlite"
        url      = "file:dev.db"
    }
"#;

pub const POSTGRES_SOURCE: &str = r#"
    datasource db {
        provider = "postgres"
        url      = "postgresql://localhost:5432"
    }
"#;

pub const MYSQL_SOURCE: &str = r#"
    datasource db {
        provider = "mysql"
        url      = "mysql://localhost:3306"
    }
"#;
