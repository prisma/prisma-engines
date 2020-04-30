extern crate datamodel;

use self::datamodel::IndexDefinition;
use datamodel::{common::ScalarType, dml, error::*};
use datamodel_connector::ScalarFieldType;
use pretty_assertions::assert_eq;

pub trait FieldAsserts {
    fn assert_base_type(&self, t: &ScalarType) -> &Self;
    fn assert_enum_type(&self, en: &str) -> &Self;
    fn assert_connector_type(&self, sft: &ScalarFieldType) -> &Self;
    fn assert_relation_name(&self, t: &str) -> &Self;
    fn assert_relation_to(&self, t: &str) -> &Self;
    fn assert_relation_delete_strategy(&self, t: dml::OnDeleteStrategy) -> &Self;
    fn assert_relation_to_fields(&self, t: &[&str]) -> &Self;
    fn assert_relation_base_fields(&self, t: &[&str]) -> &Self;
    fn assert_arity(&self, arity: &dml::FieldArity) -> &Self;
    fn assert_with_db_name(&self, t: &str) -> &Self;
    fn assert_with_documentation(&self, t: &str) -> &Self;
    fn assert_default_value(&self, t: dml::DefaultValue) -> &Self;
    fn assert_is_generated(&self, b: bool) -> &Self;
    fn assert_is_id(&self) -> &Self;
    fn assert_is_unique(&self, b: bool) -> &Self;
    fn assert_is_updated_at(&self, b: bool) -> &Self;
}

pub trait ModelAsserts {
    fn assert_has_field(&self, t: &str) -> &dml::Field;
    fn assert_is_embedded(&self, t: bool) -> &Self;
    fn assert_with_db_name(&self, t: &str) -> &Self;
    fn assert_with_documentation(&self, t: &str) -> &Self;
    fn assert_has_index(&self, def: IndexDefinition) -> &Self;
    fn assert_has_id_fields(&self, fields: &[&str]) -> &Self;
}

pub trait EnumAsserts {
    fn assert_has_value(&self, t: &str) -> &Self;
}

pub trait DatamodelAsserts {
    fn assert_has_model(&self, t: &str) -> &dml::Model;
    fn assert_has_enum(&self, t: &str) -> &dml::Enum;
}

pub trait ErrorAsserts {
    fn assert_is(&self, error: DatamodelError) -> &Self;
    fn assert_is_at(&self, index: usize, error: DatamodelError) -> &Self;
    fn assert_length(&self, length: usize) -> &Self;
}

impl FieldAsserts for dml::Field {
    fn assert_base_type(&self, t: &ScalarType) -> &Self {
        if let dml::FieldType::Base(base_type, _) = &self.field_type {
            assert_eq!(base_type, t);
        } else {
            panic!("Scalar expected, but found {:?}", self.field_type);
        }

        self
    }

    fn assert_connector_type(&self, sft: &ScalarFieldType) -> &Self {
        if let dml::FieldType::ConnectorSpecific(t) = &self.field_type {
            assert_eq!(t, sft);
        } else {
            panic!("Connector Specific Type expected, but found {:?}", self.field_type);
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

    fn assert_relation_to(&self, t: &str) -> &Self {
        if let dml::FieldType::Relation(info) = &self.field_type {
            assert_eq!(info.to, t);
        } else {
            panic!("Relation expected, but found {:?}", self.field_type);
        }

        self
    }

    fn assert_relation_name(&self, t: &str) -> &Self {
        if let dml::FieldType::Relation(info) = &self.field_type {
            assert_eq!(info.name, t.to_owned());
        } else {
            panic!("Relation expected, but found {:?}", self.field_type);
        }

        self
    }

    fn assert_relation_delete_strategy(&self, t: dml::OnDeleteStrategy) -> &Self {
        if let dml::FieldType::Relation(info) = &self.field_type {
            assert_eq!(info.on_delete, t);
        } else {
            panic!("Relation expected, but found {:?}", self.field_type);
        }

        self
    }

    fn assert_relation_base_fields(&self, t: &[&str]) -> &Self {
        if let dml::FieldType::Relation(info) = &self.field_type {
            assert_eq!(info.fields, t);
        } else {
            panic!("Relation expected, but found {:?}", self.field_type);
        }

        self
    }

    fn assert_relation_to_fields(&self, t: &[&str]) -> &Self {
        if let dml::FieldType::Relation(info) = &self.field_type {
            assert_eq!(info.to_fields, t);
        } else {
            panic!("Relation expected, but found {:?}", self.field_type);
        }

        self
    }

    fn assert_arity(&self, arity: &dml::FieldArity) -> &Self {
        assert_eq!(self.arity, *arity);

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

    fn assert_default_value(&self, t: dml::DefaultValue) -> &Self {
        assert_eq!(self.default_value, Some(t));

        self
    }

    fn assert_is_id(&self) -> &Self {
        assert!(self.is_id);

        self
    }

    fn assert_is_generated(&self, b: bool) -> &Self {
        assert_eq!(self.is_generated, b);

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

impl DatamodelAsserts for dml::Datamodel {
    fn assert_has_model(&self, t: &str) -> &dml::Model {
        self.find_model(&t.to_owned())
            .expect(format!("Model {} not found", t).as_str())
    }
    fn assert_has_enum(&self, t: &str) -> &dml::Enum {
        self.find_enum(&t.to_owned())
            .expect(format!("Enum {} not found", t).as_str())
    }
}

impl ModelAsserts for dml::Model {
    fn assert_has_field(&self, t: &str) -> &dml::Field {
        self.find_field(&t.to_owned())
            .expect(format!("Field {} not found", t).as_str())
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
    fn assert_has_value(&self, t: &str) -> &Self {
        let pred = t.to_owned();
        self.values
            .iter()
            .find(|x| *x.name == pred)
            .expect(format!("Field {} not found", t).as_str());

        self
    }
}

impl ErrorAsserts for ErrorCollection {
    fn assert_is(&self, error: DatamodelError) -> &Self {
        if self.errors.len() == 1 {
            assert_eq!(self.errors[0], error);
        } else {
            panic!("Expected exactly one validation error. Errors are: {:?}", &self);
        }

        self
    }

    fn assert_is_at(&self, index: usize, error: DatamodelError) -> &Self {
        assert_eq!(self.errors[index], error);
        self
    }

    fn assert_length(&self, length: usize) -> &Self {
        if self.errors.len() == length {
            self
        } else {
            panic!(
                "Expected exactly {} validation errors, but got {}. The errors were {:?}",
                length,
                self.errors.len(),
                &self.errors,
            );
        }
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

pub const SQLITE_SOURCE: &'static str = r#"
    datasource db {
        provider = "sqlite"
        url      = "file:dev.db"
    }
"#;

pub const POSTGRES_SOURCE: &'static str = r#"
    datasource db {
        provider = "postgres"
        url      = "postgresql://localhost:5432"
    }
"#;

pub const MYSQL_SOURCE: &'static str = r#"
    datasource db {
        provider = "mysql"
        url      = "mysql://localhost:3306"
    }
"#;
