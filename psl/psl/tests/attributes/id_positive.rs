use crate::{common::*, with_header, Provider};

#[test]
fn int_id_without_default_should_have_strategy_none() {
    let dml = indoc! {r#"
        model Model {
          id Int @id
        }
    "#};

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model.assert_has_scalar_field("id").assert_is_id(user_model);
}

#[test]
fn int_id_with_default_autoincrement_should_have_strategy_auto() {
    let dml = indoc! {r#"
        model Model {
          id Int @id @default(autoincrement())
        }
    "#};

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model.assert_has_scalar_field("id").assert_is_id(user_model);
}

#[test]
fn should_allow_string_ids_with_cuid() {
    let dml = indoc! {r#"
        model Model {
          id String @id @default(cuid())
        }
    "#};

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_scalar_field("id")
        .assert_is_id(user_model)
        .assert_base_type(&ScalarType::String)
        .assert_default_value(DefaultValue::new_expression(ValueGenerator::new_cuid()));
}

#[test]
fn should_allow_string_ids_with_uuid() {
    let dml = indoc! {r#"
        model Model {
          id String @id @default(uuid())
        }
    "#};

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_scalar_field("id")
        .assert_is_id(user_model)
        .assert_base_type(&ScalarType::String)
        .assert_default_value(DefaultValue::new_expression(ValueGenerator::new_uuid()));
}

#[test]
fn should_allow_string_ids_without_default() {
    let dml = indoc! {r#"
        model Model {
          id String @id
        }
    "#};

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_scalar_field("id")
        .assert_is_id(user_model)
        .assert_base_type(&ScalarType::String);
}

#[test]
fn should_allow_string_ids_with_static_default() {
    let dml = indoc! {r#"
        model Model {
          id String @id @default("")
        }
    "#};

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_scalar_field("id")
        .assert_is_id(user_model)
        .assert_default_value(DefaultValue::new_single(PrismaValue::String(String::from(""))))
        .assert_base_type(&ScalarType::String);
}

#[test]
fn should_allow_int_ids_with_static_default() {
    let dml = indoc! {r#"
        model Model {
          id Int @id @default(0)
        }
    "#};

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_scalar_field("id")
        .assert_is_id(user_model)
        .assert_default_value(DefaultValue::new_single(PrismaValue::Int(0)))
        .assert_base_type(&ScalarType::Int);
}

#[test]
fn multi_field_ids_must_work() {
    let dml = indoc! {r#"
        model Model {
          a String
          b Int
          @@id([a,b])
        }
    "#};

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model.assert_has_pk(PrimaryKeyDefinition {
        name: None,
        db_name: None,
        fields: vec![PrimaryKeyField::new("a"), PrimaryKeyField::new("b")],
    });
}

#[test]
fn should_allow_unique_and_id_on_same_field() {
    let dml = indoc! {r#"
        model Model {
          id Int @id @unique
        }
    "#};

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model.assert_has_pk(PrimaryKeyDefinition {
        name: None,
        db_name: None,
        fields: vec![PrimaryKeyField::new("id")],
    });

    user_model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("Model_id_key".to_string()),
        fields: vec![IndexField::new_in_model("id")],
        tpe: IndexType::Unique,
        defined_on_field: true,
        algorithm: None,
        clustered: None,
    });
}

#[test]
fn unnamed_and_unmapped_multi_field_ids_must_work() {
    let dml = with_header(
        indoc! {r#"
        model Model {
          a String
          b Int
          @@id([a,b])
        }
    "#},
        Provider::Postgres,
        &[],
    );

    let datamodel = parse(&dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model.assert_has_id_fields(&["a", "b"]);
    user_model.assert_has_named_pk("Model_pkey");
}

#[test]
fn unmapped_singular_id_must_work() {
    let dml = with_header(
        indoc! {r#"
        model Model {
          a String @id
        }
    "#},
        Provider::Postgres,
        &[],
    );

    let datamodel = parse(&dml);
    let model = datamodel.assert_has_model("Model");
    model.assert_has_id_fields(&["a"]);
    model.assert_has_named_pk("Model_pkey");
}

#[test]
fn named_multi_field_ids_must_work() {
    let dml = with_header(
        indoc! {r#"
        model Model {
          a String
          b Int
          @@id([a,b], name: "compoundId")
        }
    "#},
        Provider::Postgres,
        &[],
    );

    let datamodel = parse(&dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model.assert_has_id_fields(&["a", "b"]);
    user_model.assert_has_named_pk("Model_pkey");
}

#[test]
fn mapped_multi_field_ids_must_work() {
    let dml = with_header(
        indoc! {r#"
        model Model {
          a String
          b Int
          @@id([a,b], map:"dbname")
        }
    "#},
        Provider::Postgres,
        &[],
    );

    let datamodel = parse(&dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model.assert_has_id_fields(&["a", "b"]);
    user_model.assert_has_named_pk("dbname");
}

#[test]
fn mapped_singular_id_must_work() {
    let dml = with_header(
        indoc! {r#"
        model Model {
          a String @id(map: "test")
        }

        model Model2 {
          a String @id(map: "test2")
        }
    "#},
        Provider::Postgres,
        &[],
    );

    let datamodel = parse(&dml);
    let model = datamodel.assert_has_model("Model");
    model.assert_has_id_fields(&["a"]);
    model.assert_has_named_pk("test");

    let model2 = datamodel.assert_has_model("Model2");
    model2.assert_has_id_fields(&["a"]);
    model2.assert_has_named_pk("test2");
}

#[test]
fn named_and_mapped_multi_field_ids_must_work() {
    let dml = with_header(
        indoc! {r#"
        model Model {
          a String
          b Int
          @@id([a,b], name: "compoundId", map:"dbname")
        }
    "#},
        Provider::Postgres,
        &[],
    );

    let datamodel = parse(&dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model.assert_has_id_fields(&["a", "b"]);
    user_model.assert_has_named_pk("dbname");
}

#[test]
fn id_accepts_length_arg_on_mysql() {
    let dml = with_header(
        r#"
     model User {
         firstName  String
         middleName String
         lastName   String
         
         @@id([firstName, middleName(length: 1), lastName])
     }
     
     model Blog {
         title  String @id(length:5)
     }
     "#,
        Provider::Mysql,
        &[],
    );

    let schema = parse(&dml);
    let user_model = schema.assert_has_model("User");
    let blog_model = schema.assert_has_model("Blog");

    user_model.assert_has_pk(PrimaryKeyDefinition {
        name: None,
        db_name: None,
        fields: vec![
            PrimaryKeyField {
                name: "firstName".to_string(),
                sort_order: None,
                length: None,
            },
            PrimaryKeyField {
                name: "middleName".to_string(),
                sort_order: None,
                length: Some(1),
            },
            PrimaryKeyField {
                name: "lastName".to_string(),
                sort_order: None,
                length: None,
            },
        ],
    });

    blog_model.assert_has_pk(PrimaryKeyDefinition {
        name: None,
        db_name: None,
        fields: vec![PrimaryKeyField {
            name: "title".to_string(),
            sort_order: None,
            length: Some(5),
        }],
    });
}

#[test]
fn id_accepts_sort_arg_on_sqlserver() {
    let dml = with_header(
        r#"
     model User {
         firstName  String
         middleName String
         lastName   String
         
         @@id([firstName, middleName(sort: Desc), lastName])
     }
     
     model Blog {
         title  String @id(sort: Desc)
     }
     "#,
        Provider::SqlServer,
        &[],
    );

    let schema = parse(&dml);
    let user_model = schema.assert_has_model("User");
    let blog_model = schema.assert_has_model("Blog");

    user_model.assert_has_pk(PrimaryKeyDefinition {
        name: None,
        db_name: Some("User_pkey".to_string()),
        fields: vec![
            PrimaryKeyField {
                name: "firstName".to_string(),
                sort_order: None,
                length: None,
            },
            PrimaryKeyField {
                name: "middleName".to_string(),
                sort_order: Some(SortOrder::Desc),
                length: None,
            },
            PrimaryKeyField {
                name: "lastName".to_string(),
                sort_order: None,
                length: None,
            },
        ],
    });

    blog_model.assert_has_pk(PrimaryKeyDefinition {
        name: None,
        db_name: Some("Blog_pkey".to_string()),
        fields: vec![PrimaryKeyField {
            name: "title".to_string(),
            sort_order: Some(SortOrder::Desc),
            length: None,
        }],
    });
}

#[test]
fn mysql_allows_id_length_prefix() {
    let dml = indoc! {r#"
        model A {
          id String @id(length: 30) @test.VarChar(255)
        }
    "#};
    let schema = with_header(dml, Provider::Mysql, &[]);
    assert_valid(&schema);
}

#[test]
fn mysql_allows_compound_id_length_prefix() {
    let dml = indoc! {r#"
        model A {
          a String @test.VarChar(255)
          b String @test.VarChar(255)

          @@id([a(length: 10), b(length: 20)])
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &[]);
    assert_valid(&schema);
}

#[test]
fn mssql_allows_id_sort_argument() {
    let dml = indoc! {r#"
        model A {
          id Int @id(sort: Desc)
        }
    "#};

    let schema = with_header(dml, Provider::SqlServer, &[]);
    assert_valid(&schema);
}

#[test]
fn mssql_allows_compound_id_sort_argument() {
    let dml = indoc! {r#"
        model A {
          a String @test.VarChar(255)
          b String @test.VarChar(255)

          @@id([a(sort: Asc), b(sort: Desc)])
        }
    "#};

    let schema = with_header(dml, Provider::SqlServer, &[]);
    assert_valid(&schema);
}

#[test]
fn mongodb_compound_unique_can_have_id_as_part_of_it() {
    let dml = indoc! {r#"
        model User {
          id String @id @map("_id") @test.ObjectId
          di Int

          @@unique([id, di])
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    assert_valid(&schema);
}
