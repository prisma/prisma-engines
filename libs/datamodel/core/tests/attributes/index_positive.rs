use datamodel::{render_datamodel_to_string, IndexDefinition, IndexType};

use crate::attributes::with_named_constraints;
use crate::common::*;

#[test]
fn basic_index_must_work() {
    let dml = r#"
    model User {
        id        Int    @id
        firstName String
        lastName  String

        @@index([firstName,lastName])
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("User.firstName_lastName_index".to_string()),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
    });
}

#[test]
fn indexes_on_enum_fields_must_work() {
    let dml = r#"
    model User {
        id        Int    @id
        role      Role

        @@index([role])
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
        db_name: Some("User.role_index".to_string()),
        fields: vec!["role".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
    });
}

#[test]
fn the_name_argument_must_work() {
    let dml = r#"
    model User {
        id        Int    @id
        firstName String
        lastName  String

        @@index([firstName,lastName], name: "MyIndexName")
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("MyIndexName".to_string()),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
    });
}

#[test]
fn the_map_argument_must_work_with_preview_flag() {
    let dml = r#"
     datasource test {
        provider = "postgres"
        url = "postgresql://..."
     }
        
     generator js {
        provider = "prisma-client-js"
        previewFeatures = ["NamedConstraints"]
     }
    
     model User {
         id        Int    @id
         firstName String
         lastName  String

         @@index([firstName,lastName], map: "MyIndexName")
     }
     "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("MyIndexName".to_string()),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
    });
}

#[test]
fn multiple_indexes_with_same_name_are_supported_by_mysql() {
    let dml = r#"
    datasource mysql {
        provider = "mysql"
        url = "mysql://asdlj"
    }

    model User {
        id         Int @id
        neighborId Int

        @@index([id], name: "MyIndexName")
     }

     model Post {
        id Int @id
        optionId Int

        @@index([id], name: "MyIndexName")
     }
    "#;

    let schema = parse(dml);

    let user_model = schema.assert_has_model("User");
    let post_model = schema.assert_has_model("Post");

    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("MyIndexName".to_string()),
        fields: vec!["id".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
    });

    post_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("MyIndexName".to_string()),
        fields: vec!["id".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
    });
}

#[test]
fn multiple_index_must_work() {
    let dml = r#"
    model User {
        id        Int    @id
        firstName String
        lastName  String

        @@index([firstName,lastName])
        @@index([firstName,lastName], name: "MyIndexName")
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");

    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("User.firstName_lastName_index".to_string()),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
    });

    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("MyIndexName".to_string()),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
    });
}

#[test]
fn index_attributes_must_serialize_to_valid_dml() {
    let dml = r#"
        model User {
            id        Int    @id
            firstName String
            lastName  String

            @@index([firstName,lastName], name: "customName")
        }
    "#;
    let schema = parse(dml);

    assert!(datamodel::parse_datamodel(&render_datamodel_to_string(&schema, None)).is_ok());
}

#[test]
fn index_accepts_three_different_notations() {
    let dml = with_named_constraints(
        r#"
    model User {
        id        Int    @id
        firstName String
        lastName  String

        // compatibility
        @@index([firstName,lastName], map: "OtherIndexName")
        //explicit
        @@index([firstName,lastName], name: "MyIndexName")
        //implicit
        @@index([firstName,lastName])
    }
    "#,
    );

    let schema = parse(&dml);
    let user_model = schema.assert_has_model("User");

    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("OtherIndexName".to_string()),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
    });

    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("MyIndexName".to_string()),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
    });

    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("User_firstName_lastName_idx".to_string()),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
    });
}
