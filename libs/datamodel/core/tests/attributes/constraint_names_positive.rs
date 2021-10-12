use crate::common::{parse, DatamodelAsserts, ModelAsserts};
use datamodel::{IndexDefinition, IndexType};

#[test]
fn multiple_indexes_with_same_name_on_different_models_are_supported_by_mysql() {
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
fn foreign_keys_and_indexes_with_same_name_on_same_table_are_not_supported_on_mysql() {
    let dml = r#"
        datasource mysql {
            provider = "mysql"
            url = "mysql://asdlj"
        }

        model A {
          id  Int @id
          bId Int
          b   B   @relation(fields: [bId], references: [id], map: "foo")
          
          @@index([bId], map: "foo")
        }
        
        model B {
          id Int @id
          as A[]
        }
    "#;

    let schema = parse(dml);

    let a = schema.assert_has_model("A");

    a.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("foo".to_string()),
        fields: vec!["bId".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
    });
}

#[test]
fn multiple_indexes_with_same_name_on_different_models_are_supported_by_mssql() {
    let dml = r#"
    datasource sqlserver {
        provider = "sqlserver"
        url = "sqlserver://asdlj"
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
fn multiple_constraints_with_same_name_in_different_namespaces_are_supported_by_mssql() {
    let dml = r#"
    datasource sqlserver {
        provider = "sqlserver"
        url = "sqlserver://asdlj"
    }

    model User {
        id         Int @id
        neighborId Int @default(5, map: "MyName")
        posts      Post[]

        @@index([id], name: "MyName")
     }

     model Post {
        id Int @id
        userId Int
        User   User @relation(fields:[userId], references:[id], map: "MyOtherName")

        @@index([id], name: "MyOtherName")
     }
    "#;

    let schema = parse(dml);

    let user_model = schema.assert_has_model("User");
    let post_model = schema.assert_has_model("Post");

    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("MyName".to_string()),
        fields: vec!["id".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
    });

    post_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("MyOtherName".to_string()),
        fields: vec!["id".to_string()],
        tpe: IndexType::Normal,
        defined_on_field: false,
    });
}
