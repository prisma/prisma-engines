use crate::common::*;
use datamodel::{render_datamodel_to_string, IndexDefinition, IndexType};

#[test]
fn basic_unique_index_must_work() {
    let dml = r#"
    model User {
        id        Int    @id
        firstName String
        lastName  String

        @@unique([firstName,lastName])
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model.assert_has_index(IndexDefinition {
        name: None,
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Unique,
    });
}

#[test]
fn must_succeed_on_model_with_unique_criteria() {
    let dml1 = r#"
    model Model {
        id String @id
    }
    "#;
    let _ = parse(dml1);

    let dml2 = r#"
    model Model {
        a String
        b String
        @@id([a,b])
    }
    "#;
    let _ = parse(dml2);

    let dml3 = r#"
    model Model {
        unique String @unique
    }
    "#;
    let _ = parse(dml3);

    let dml4 = r#"
    model Model {
        a String
        b String
        @@unique([a,b])
    }
    "#;
    let _ = parse(dml4);
}

#[test]
fn single_field_unique_on_enum_field_must_work() {
    let dml = r#"
    model User {
        id        Int    @id
        role      Role   @unique
    }

    enum Role {
        Admin
        Member
    }
    "#;

    let schema = parse(dml);
    schema
        .assert_has_model("User")
        .assert_has_scalar_field("role")
        .assert_is_unique(true);
}

#[test]
fn the_name_argument_must_work() {
    let dml = r#"
    model User {
        id        Int    @id
        firstName String
        lastName  String

        @@unique([firstName,lastName], name: "MyIndexName")
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model.assert_has_index(IndexDefinition {
        name: Some("MyIndexName".to_string()),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Unique,
    });
}

#[test]
fn multiple_unique_must_work() {
    let dml = r#"
    model User {
        id        Int    @id
        firstName String
        lastName  String

        @@unique([firstName,lastName])
        @@unique([firstName,lastName], name: "MyIndexName")
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");

    user_model.assert_has_index(IndexDefinition {
        name: None,
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Unique,
    });

    user_model.assert_has_index(IndexDefinition {
        name: Some("MyIndexName".to_string()),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Unique,
    });
}

#[test]
fn multi_field_unique_on_native_type_fields_fields_must_work() {
    let dml = r#"
    datasource ds {
        provider = "mysql"
        url = "mysql://"
    }

    model User {
        id        Int    @id
        role      Bytes
        role2     Bytes @ds.VarBinary(40)

        @@unique([role2, role])
    }
    "#;
    parse(dml);
}

#[test]
fn multi_field_unique_indexes_on_enum_fields_must_work() {
    let dml = r#"
    model User {
        id        Int    @id
        role      Role

        @@unique([role])
    }

    enum Role {
        Admin
        Member
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model.assert_has_index(IndexDefinition {
        name: None,
        fields: vec!["role".to_string()],
        tpe: IndexType::Unique,
    });
}

#[test]
fn unique_attributes_must_serialize_to_valid_dml() {
    let dml = r#"
        model User {
            id        Int    @id
            firstName String
            lastName  String

            @@unique([firstName,lastName], name: "customName")
        }
    "#;
    let schema = parse(dml);

    assert!(datamodel::parse_datamodel(&render_datamodel_to_string(&schema)).is_ok());
}
