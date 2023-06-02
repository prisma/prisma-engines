use crate::{common::*, with_header, Provider};
use psl::builtin_connectors::MongoDbType;

#[test]
fn valid_json_usage_in_model() {
    let dml = indoc! {r#"
        model A {
          id Int  @id            @map("_id")
          a  Json @test.Json
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let datamodel = psl::parse_schema(schema).unwrap();
    let model = datamodel.assert_has_model("A");

    model
        .assert_has_scalar_field("a")
        .assert_native_type(datamodel.connector, &MongoDbType::Json);
}

#[test]
fn valid_object_id_usage_in_model() {
    let dml = indoc! {r#"
        model A {
          id Int    @id            @map("_id")
          a  String @test.ObjectId
          b  Bytes  @test.ObjectId
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let datamodel = psl::parse_schema(schema).unwrap();
    let model = datamodel.assert_has_model("A");

    model
        .assert_has_scalar_field("a")
        .assert_native_type(datamodel.connector, &MongoDbType::ObjectId);

    model
        .assert_has_scalar_field("b")
        .assert_native_type(datamodel.connector, &MongoDbType::ObjectId);
}

#[test]
fn valid_long_usage_in_model() {
    let dml = indoc! {r#"
        model A {
          id Int    @id            @map("_id")
          a  Int    @test.Long
          b  BigInt @test.Long
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let datamodel = psl::parse_schema(schema).unwrap();
    let model = datamodel.assert_has_model("A");

    model
        .assert_has_scalar_field("a")
        .assert_native_type(datamodel.connector, &MongoDbType::Long);

    model
        .assert_has_scalar_field("b")
        .assert_native_type(datamodel.connector, &MongoDbType::Long);
}
