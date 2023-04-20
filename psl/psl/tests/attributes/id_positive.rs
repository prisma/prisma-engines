use psl::parser_database::ScalarType;

use crate::{common::*, with_header, Provider};

#[test]
fn int_id_without_default_should_have_strategy_none() {
    let dml = indoc! {r#"
        model Model {
          id Int @id
        }
    "#};

    psl::parse_schema(dml)
        .unwrap()
        .assert_has_model("Model")
        .assert_id_on_fields(&["id"]);
}

#[test]
fn int_id_with_default_autoincrement_should_have_strategy_auto() {
    let dml = indoc! {r#"
        model Model {
          id Int @id @default(autoincrement())
        }
    "#};

    psl::parse_schema(dml)
        .unwrap()
        .assert_has_model("Model")
        .assert_id_on_fields(&["id"]);
}

#[test]
fn should_allow_string_ids_with_cuid() {
    let dml = indoc! {r#"
        model Model {
          id String @id @default(cuid())
        }
    "#};

    let schema = psl::parse_schema(dml).unwrap();
    let model = schema.assert_has_model("Model");

    model
        .assert_has_scalar_field("id")
        .assert_scalar_type(ScalarType::String)
        .assert_default_value()
        .assert_cuid();

    model.assert_id_on_fields(&["id"]);
}

#[test]
fn should_allow_string_ids_with_uuid() {
    let dml = indoc! {r#"
        model Model {
          id String @id @default(uuid())
        }
    "#};

    let schema = psl::parse_schema(dml).unwrap();
    let model = schema.assert_has_model("Model");

    model
        .assert_has_scalar_field("id")
        .assert_scalar_type(ScalarType::String)
        .assert_default_value()
        .assert_uuid();

    model.assert_id_on_fields(&["id"]);
}

#[test]
fn should_allow_string_ids_without_default() {
    let dml = indoc! {r#"
        model Model {
          id String @id
        }
    "#};

    let schema = psl::parse_schema(dml).unwrap();
    let model = schema.assert_has_model("Model");
    model.assert_id_on_fields(&["id"]);

    model
        .assert_has_scalar_field("id")
        .assert_scalar_type(ScalarType::String);
}

#[test]
fn should_allow_string_ids_with_static_default() {
    let dml = indoc! {r#"
        model Model {
          id String @id @default("")
        }
    "#};

    let schema = psl::parse_schema(dml).unwrap();
    let model = schema.assert_has_model("Model");
    model.assert_id_on_fields(&["id"]);

    model
        .assert_has_scalar_field("id")
        .assert_scalar_type(ScalarType::String)
        .assert_default_value()
        .assert_string("");
}

#[test]
fn should_allow_int_ids_with_static_default() {
    let dml = indoc! {r#"
        model Model {
          id Int @id @default(0)
        }
    "#};

    let schema = psl::parse_schema(dml).unwrap();
    let model = schema.assert_has_model("Model");
    model.assert_id_on_fields(&["id"]);

    model
        .assert_has_scalar_field("id")
        .assert_scalar_type(ScalarType::Int)
        .assert_default_value()
        .assert_int(0);
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

    let schema = psl::parse_schema(dml).unwrap();
    let model = schema.assert_has_model("Model");
    model.assert_id_on_fields(&["a", "b"]);
}

#[test]
fn should_allow_unique_and_id_on_same_field() {
    let dml = indoc! {r#"
        model Model {
          id Int @id @unique
        }
    "#};

    let schema = psl::parse_schema(dml).unwrap();
    let model = schema.assert_has_model("Model");

    model.assert_id_on_fields(&["id"]);
    model.assert_unique_on_fields(&["id"]);
}

#[test]
fn named_multi_field_ids_must_work() {
    let dml = indoc! {r#"
        model Model {
          a String
          b Int
          @@id([a,b], name: "compoundId")
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("Model")
        .assert_id_on_fields(&["a", "b"])
        .assert_name("compoundId");
}

#[test]
fn mapped_multi_field_ids_must_work() {
    let dml = indoc! {r#"
        model Model {
          a String
          b Int
          @@id([a,b], map: "dbname")
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("Model")
        .assert_id_on_fields(&["a", "b"])
        .assert_mapped_name("dbname");
}

#[test]
fn mapped_singular_id_must_work() {
    let dml = indoc! {r#"
        model Model {
          a String @id(map: "test")
        }

        model Model2 {
          a String @id(map: "test2")
        }
    "#};

    let datamodel = psl::parse_schema(with_header(dml, Provider::Postgres, &[])).unwrap();

    datamodel
        .assert_has_model("Model")
        .assert_id_on_fields(&["a"])
        .assert_mapped_name("test");

    datamodel
        .assert_has_model("Model2")
        .assert_id_on_fields(&["a"])
        .assert_mapped_name("test2");
}

#[test]
fn named_and_mapped_multi_field_ids_must_work() {
    let dml = indoc! {r#"
        model Model {
          a String
          b Int
          @@id([a,b], name: "compoundId", map:"dbname")
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("Model")
        .assert_id_on_fields(&["a", "b"])
        .assert_mapped_name("dbname")
        .assert_name("compoundId");
}

#[test]
fn id_accepts_length_arg_on_mysql() {
    let dml = indoc! {r#"
        model User {
          firstName  String
          middleName String
          lastName   String
         
          @@id([firstName, middleName(length: 1), lastName])
         }
     
         model Blog {
          title  String @id(length:5)
         }
     "#};

    let schema = psl::parse_schema(with_header(dml, Provider::Mysql, &[])).unwrap();

    schema
        .assert_has_model("User")
        .assert_id_on_fields(&["firstName", "middleName", "lastName"])
        .assert_field("middleName")
        .assert_length(1);

    schema
        .assert_has_model("Blog")
        .assert_id_on_fields(&["title"])
        .assert_field("title")
        .assert_length(5);
}

#[test]
fn id_accepts_sort_arg_on_sqlserver() {
    let dml = indoc! {r#"
        model User {
          firstName  String
          middleName String
          lastName   String
         
          @@id([firstName, middleName(sort: Desc), lastName])
        }
     
        model Blog {
          title  String @id(sort: Desc)
        }
    "#};

    let schema = psl::parse_schema(with_header(dml, Provider::SqlServer, &[])).unwrap();

    schema
        .assert_has_model("User")
        .assert_id_on_fields(&["firstName", "middleName", "lastName"])
        .assert_field("middleName")
        .assert_descending();

    schema
        .assert_has_model("Blog")
        .assert_id_on_fields(&["title"])
        .assert_field("title")
        .assert_descending();
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
