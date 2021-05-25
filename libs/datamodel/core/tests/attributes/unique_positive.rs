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
        name_in_client: None,
        name_in_db_matches_default: true,
        name_in_db: "User_firstName_lastName_key".to_string(),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Unique,
    });
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
        name_in_db: "User_firstName_lastName_key".to_string(),
        name_in_db_matches_default: true,
        name_in_client: Some("MyIndexName".to_string()),
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
        @@unique([firstName,lastName], map: "MyIndexName")
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");

    user_model.assert_has_index(IndexDefinition {
        name_in_db: "User_firstName_lastName_key".to_string(),
        name_in_db_matches_default: true,
        name_in_client: None,
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Unique,
    });

    user_model.assert_has_index(IndexDefinition {
        name_in_db: "MyIndexName".to_string(),
        name_in_db_matches_default: false,
        name_in_client: None,
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Unique,
    });
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
        name_in_client: None,
        name_in_db_matches_default: true,
        name_in_db: "User_role_key".to_string(),
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

#[test]
fn mapped_multi_field_unique_must_work() {
    let dml = r#"
    model Model {
        a String
        b Int
        @@unique([a,b], map:"dbname")
    }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model.assert_has_index(IndexDefinition {
        name_in_client: None,
        name_in_db: "dbname".to_string(),
        name_in_db_matches_default: false,
        fields: vec!["a".to_string(), "b".to_string()],
        tpe: IndexType::Unique,
    });
}

#[test]
fn mapped_singular_unique_must_work() {
    let dml = r#"
    model Model {
        a String @unique("test")
    }
    
    model Model2 {
        a String @unique(map: "test2")
    }
    "#;

    let datamodel = parse(dml);
    let model = datamodel.assert_has_model("Model");
    model.assert_has_index(IndexDefinition {
        name_in_client: None,
        name_in_db: "test".to_string(),
        name_in_db_matches_default: false,
        fields: vec!["a".to_string()],
        tpe: IndexType::Unique,
    });

    let model2 = datamodel.assert_has_model("Model2");
    model2.assert_has_index(IndexDefinition {
        name_in_client: None,
        name_in_db: "test2".to_string(),
        name_in_db_matches_default: false,
        fields: vec!["a".to_string()],
        tpe: IndexType::Unique,
    });
}

#[test]
fn named_and_mapped_multi_field_unique_must_work() {
    let dml = r#"
    model Model {
        a String
        b Int
        @@unique([a,b], name: "compoundId", map:"dbname")
    }
    "#;

    let datamodel = parse(dml);
    let model = datamodel.assert_has_model("Model");
    model.assert_has_index(IndexDefinition {
        name_in_client: Some("compoundId".to_string()),
        name_in_db: "dbname".to_string(),
        name_in_db_matches_default: false,
        fields: vec!["a".to_string(), "b".to_string()],
        tpe: IndexType::Unique,
    });
}
