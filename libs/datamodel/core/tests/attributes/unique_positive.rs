use crate::attributes::with_postgres_provider;
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
        db_name: Some("User_firstName_lastName_key".to_string()),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Unique,
        defined_on_field: false,
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
    let model = schema.assert_has_model("User");
    model.assert_has_scalar_field("role");
    assert!(model.field_is_unique("role"));
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
        db_name: Some("User_firstName_lastName_key".to_string()),
        fields: vec!["firstName".to_string(), "lastName".to_string()],
        tpe: IndexType::Unique,
        defined_on_field: false,
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
            @@unique([firstName,lastName], name: "MyIndexName", map: "MyIndexName")
        }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");

    let expect = expect![[r#"
        [
            IndexDefinition {
                name: None,
                db_name: Some(
                    "User_firstName_lastName_key",
                ),
                fields: [
                    "firstName",
                    "lastName",
                ],
                tpe: Unique,
                defined_on_field: false,
            },
            IndexDefinition {
                name: Some(
                    "MyIndexName",
                ),
                db_name: Some(
                    "MyIndexName",
                ),
                fields: [
                    "firstName",
                    "lastName",
                ],
                tpe: Unique,
                defined_on_field: false,
            },
        ]
    "#]];

    expect.assert_debug_eq(&user_model.indices);
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
        db_name: Some("User_role_key".to_string()),
        fields: vec!["role".to_string()],
        tpe: IndexType::Unique,
        defined_on_field: false,
    });
}

#[test]
fn single_field_unique_indexes_on_enum_fields_must_work() {
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
    let user_model = schema.assert_has_model("User");
    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("User_role_key".to_string()),
        fields: vec!["role".to_string()],
        tpe: IndexType::Unique,
        defined_on_field: true,
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

    assert!(datamodel::parse_datamodel(&render_datamodel_to_string(&schema, None)).is_ok());
}

#[test]
fn named_multi_field_unique_must_work() {
    //Compatibility case
    let dml = with_postgres_provider(
        r#"
     model User {
         a String
         b Int
         @@unique([a,b], name:"ClientName")
     }
     "#,
    );

    let datamodel = parse(&dml);
    let user_model = datamodel.assert_has_model("User");
    user_model.assert_has_index(IndexDefinition {
        name: Some("ClientName".to_string()),
        db_name: Some("User_a_b_key".to_string()),
        fields: vec!["a".to_string(), "b".to_string()],
        tpe: IndexType::Unique,
        defined_on_field: false,
    });
}

#[test]
fn mapped_multi_field_unique_must_work() {
    let dml = with_postgres_provider(
        r#"
     model User {
         a String
         b Int
         @@unique([a,b], map:"dbname")
     }
     "#,
    );

    let datamodel = parse(&dml);
    let user_model = datamodel.assert_has_model("User");
    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("dbname".to_string()),
        fields: vec!["a".to_string(), "b".to_string()],
        tpe: IndexType::Unique,
        defined_on_field: false,
    });
}

#[test]
fn mapped_singular_unique_must_work() {
    let dml = with_postgres_provider(
        r#"
     model Model {
         a String @unique(map: "test")
     }
     
     model Model2 {
         a String @unique(map: "test2")
     }
     "#,
    );

    let datamodel = parse(&dml);
    let model = datamodel.assert_has_model("Model");
    model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("test".to_string()),
        fields: vec!["a".to_string()],
        tpe: IndexType::Unique,
        defined_on_field: true,
    });

    let model2 = datamodel.assert_has_model("Model2");
    model2.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("test2".to_string()),
        fields: vec!["a".to_string()],
        tpe: IndexType::Unique,
        defined_on_field: true,
    });
}

#[test]
fn named_and_mapped_multi_field_unique_must_work() {
    let dml = with_postgres_provider(
        r#"
     model Model {
         a String
         b Int
         @@unique([a,b], name: "compoundId", map:"dbname")
     }
     "#,
    );

    let datamodel = parse(&dml);
    let model = datamodel.assert_has_model("Model");
    model.assert_has_index(IndexDefinition {
        name: Some("compoundId".to_string()),
        db_name: Some("dbname".to_string()),
        fields: vec!["a".to_string(), "b".to_string()],
        tpe: IndexType::Unique,
        defined_on_field: false,
    });
}

#[test]
fn implicit_names_must_work() {
    let dml = with_postgres_provider(
        r#"
     model Model {
         a String @unique
         b Int
         @@unique([a,b])
     }
     "#,
    );

    let datamodel = parse(&dml);
    let model = datamodel.assert_has_model("Model");
    model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("Model_a_b_key".to_string()),
        fields: vec!["a".to_string(), "b".to_string()],
        tpe: IndexType::Unique,
        defined_on_field: false,
    });

    model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("Model_a_key".to_string()),
        fields: vec!["a".to_string()],
        tpe: IndexType::Unique,
        defined_on_field: true,
    });
}

#[test]
fn defined_on_field_must_work() {
    let dml = with_postgres_provider(
        r#"
     model Model {
         a String @unique
         b Int
         @@unique([b])
     }
     "#,
    );

    let datamodel = parse(&dml);
    let model = datamodel.assert_has_model("Model");
    model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("Model_a_key".to_string()),
        fields: vec!["a".to_string()],
        tpe: IndexType::Unique,
        defined_on_field: true,
    });

    model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("Model_b_key".to_string()),
        fields: vec!["b".to_string()],
        tpe: IndexType::Unique,
        defined_on_field: false,
    });
}

#[test]
fn mapping_unique_to_a_field_name_should_work() {
    let dml = r#"
     model User {
         used           Int
         name           String            
         identification Int

         @@unique([name, identification], name: "usedUnique", map: "used")
     }
     "#;

    let datamodel = parse(dml);
    let model = datamodel.assert_has_model("User");
    model.assert_has_index(IndexDefinition {
        name: Some("usedUnique".to_string()),
        db_name: Some("used".to_string()),
        fields: vec!["name".to_string(), "identification".to_string()],
        tpe: IndexType::Unique,
        defined_on_field: false,
    });
}
