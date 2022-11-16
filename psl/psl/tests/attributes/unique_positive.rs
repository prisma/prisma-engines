use crate::{common::*, with_header, Provider};

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
        fields: vec![
            IndexField::new_in_model("firstName"),
            IndexField::new_in_model("lastName"),
        ],
        tpe: IndexType::Unique,
        defined_on_field: false,
        algorithm: None,
        clustered: None,
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
        fields: vec![
            IndexField::new_in_model("firstName"),
            IndexField::new_in_model("lastName"),
        ],
        tpe: IndexType::Unique,
        defined_on_field: false,
        algorithm: None,
        clustered: None,
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
                    IndexField {
                        path: [
                            (
                                "firstName",
                                None,
                            ),
                        ],
                        sort_order: None,
                        length: None,
                        operator_class: None,
                    },
                    IndexField {
                        path: [
                            (
                                "lastName",
                                None,
                            ),
                        ],
                        sort_order: None,
                        length: None,
                        operator_class: None,
                    },
                ],
                tpe: Unique,
                clustered: None,
                algorithm: None,
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
                    IndexField {
                        path: [
                            (
                                "firstName",
                                None,
                            ),
                        ],
                        sort_order: None,
                        length: None,
                        operator_class: None,
                    },
                    IndexField {
                        path: [
                            (
                                "lastName",
                                None,
                            ),
                        ],
                        sort_order: None,
                        length: None,
                        operator_class: None,
                    },
                ],
                tpe: Unique,
                clustered: None,
                algorithm: None,
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
        fields: vec![IndexField::new_in_model("role")],
        tpe: IndexType::Unique,
        defined_on_field: false,
        algorithm: None,
        clustered: None,
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
        fields: vec![IndexField::new_in_model("role")],
        tpe: IndexType::Unique,
        defined_on_field: true,
        algorithm: None,
        clustered: None,
    });
}

#[test]
fn named_multi_field_unique_must_work() {
    //Compatibility case
    let dml = with_header(
        r#"
     model User {
         a String
         b Int
         @@unique([a,b], name:"ClientName")
     }
     "#,
        Provider::Postgres,
        &[],
    );

    let datamodel = parse(&dml);
    let user_model = datamodel.assert_has_model("User");
    user_model.assert_has_index(IndexDefinition {
        name: Some("ClientName".to_string()),
        db_name: Some("User_a_b_key".to_string()),
        fields: vec![IndexField::new_in_model("a"), IndexField::new_in_model("b")],
        tpe: IndexType::Unique,
        defined_on_field: false,
        algorithm: None,
        clustered: None,
    });
}

#[test]
fn mapped_multi_field_unique_must_work() {
    let dml = with_header(
        r#"
     model User {
         a String
         b Int
         @@unique([a,b], map:"dbname")
     }
     "#,
        Provider::Postgres,
        &[],
    );

    let datamodel = parse(&dml);
    let user_model = datamodel.assert_has_model("User");
    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("dbname".to_string()),
        fields: vec![IndexField::new_in_model("a"), IndexField::new_in_model("b")],
        tpe: IndexType::Unique,
        defined_on_field: false,
        algorithm: None,
        clustered: None,
    });
}

#[test]
fn mapped_singular_unique_must_work() {
    let dml = with_header(
        r#"
     model Model {
         a String @unique(map: "test")
     }
     
     model Model2 {
         a String @unique(map: "test2")
     }
     "#,
        Provider::Postgres,
        &[],
    );

    let datamodel = parse(&dml);
    let model = datamodel.assert_has_model("Model");
    model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("test".to_string()),
        fields: vec![IndexField::new_in_model("a")],
        tpe: IndexType::Unique,
        defined_on_field: true,
        algorithm: None,
        clustered: None,
    });

    let model2 = datamodel.assert_has_model("Model2");
    model2.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("test2".to_string()),
        fields: vec![IndexField::new_in_model("a")],
        tpe: IndexType::Unique,
        defined_on_field: true,
        algorithm: None,
        clustered: None,
    });
}

#[test]
fn named_and_mapped_multi_field_unique_must_work() {
    let dml = with_header(
        r#"
     model Model {
         a String
         b Int
         @@unique([a,b], name: "compoundId", map:"dbname")
     }
     "#,
        Provider::Postgres,
        &[],
    );

    let datamodel = parse(&dml);
    let model = datamodel.assert_has_model("Model");
    model.assert_has_index(IndexDefinition {
        name: Some("compoundId".to_string()),
        db_name: Some("dbname".to_string()),
        fields: vec![IndexField::new_in_model("a"), IndexField::new_in_model("b")],
        tpe: IndexType::Unique,
        defined_on_field: false,
        algorithm: None,
        clustered: None,
    });
}

#[test]
fn implicit_names_must_work() {
    let dml = with_header(
        r#"
     model Model {
         a String @unique
         b Int
         @@unique([a,b])
     }
     "#,
        Provider::Postgres,
        &[],
    );

    let datamodel = parse(&dml);
    let model = datamodel.assert_has_model("Model");
    model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("Model_a_b_key".to_string()),
        fields: vec![IndexField::new_in_model("a"), IndexField::new_in_model("b")],
        tpe: IndexType::Unique,
        defined_on_field: false,
        algorithm: None,
        clustered: None,
    });

    model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("Model_a_key".to_string()),
        fields: vec![IndexField::new_in_model("a")],
        tpe: IndexType::Unique,
        defined_on_field: true,
        algorithm: None,
        clustered: None,
    });
}

#[test]
fn defined_on_field_must_work() {
    let dml = with_header(
        r#"
     model Model {
         a String @unique
         b Int
         @@unique([b])
     }
     "#,
        Provider::Postgres,
        &[],
    );

    let datamodel = parse(&dml);
    let model = datamodel.assert_has_model("Model");
    model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("Model_a_key".to_string()),
        fields: vec![IndexField::new_in_model("a")],
        tpe: IndexType::Unique,
        defined_on_field: true,
        algorithm: None,
        clustered: None,
    });

    model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("Model_b_key".to_string()),
        fields: vec![IndexField::new_in_model("b")],
        tpe: IndexType::Unique,
        defined_on_field: false,
        algorithm: None,
        clustered: None,
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
        fields: vec![
            IndexField::new_in_model("name"),
            IndexField::new_in_model("identification"),
        ],
        tpe: IndexType::Unique,
        defined_on_field: false,
        algorithm: None,
        clustered: None,
    });
}

#[test]
fn duplicate_custom_names_on_different_model_should_work() {
    let dml = r#"
     model User {
         name           String            
         identification Int

         @@unique([name, identification], name: "duplicateUnique", map: "onUser")
     }
     
     model Post {
         name           String            
         identification Int

         @@unique([name, identification], name: "duplicateUnique", map: "onPost")
     }
     "#;

    let datamodel = parse(dml);
    let user = datamodel.assert_has_model("User");
    user.assert_has_index(IndexDefinition {
        name: Some("duplicateUnique".to_string()),
        db_name: Some("onUser".to_string()),
        fields: vec![
            IndexField::new_in_model("name"),
            IndexField::new_in_model("identification"),
        ],
        tpe: IndexType::Unique,
        defined_on_field: false,
        algorithm: None,
        clustered: None,
    });

    let post = datamodel.assert_has_model("Post");
    post.assert_has_index(IndexDefinition {
        name: Some("duplicateUnique".to_string()),
        db_name: Some("onPost".to_string()),
        fields: vec![
            IndexField::new_in_model("name"),
            IndexField::new_in_model("identification"),
        ],
        tpe: IndexType::Unique,
        defined_on_field: false,
        algorithm: None,
        clustered: None,
    });
}
