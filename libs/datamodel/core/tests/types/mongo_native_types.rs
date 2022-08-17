use crate::{common::*, with_header, Provider};
use native_types::MongoDbType;

#[test]
fn valid_json_usage_in_model() {
    let dml = indoc! {r#"
        model A {
          id Int  @id            @map("_id")
          a  Json @test.Json
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let (_, datamodel) = datamodel::parse_schema(&schema).unwrap();

    let model = datamodel.assert_has_model("A");

    let nt = model.assert_has_scalar_field("a").assert_native_type();
    let mongo_type: MongoDbType = nt.deserialize_native_type();
    assert_eq!(MongoDbType::Json, mongo_type);
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
    let (_, datamodel) = datamodel::parse_schema(&schema).unwrap();

    let model = datamodel.assert_has_model("A");

    let nt = model.assert_has_scalar_field("a").assert_native_type();
    let mongo_type: MongoDbType = nt.deserialize_native_type();
    assert_eq!(MongoDbType::ObjectId, mongo_type);

    let nt = model.assert_has_scalar_field("b").assert_native_type();
    let mongo_type: MongoDbType = nt.deserialize_native_type();
    assert_eq!(MongoDbType::ObjectId, mongo_type);
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
    let (_, datamodel) = datamodel::parse_schema(&schema).unwrap();

    let model = datamodel.assert_has_model("A");

    let nt = model.assert_has_scalar_field("a").assert_native_type();
    let mongo_type: MongoDbType = nt.deserialize_native_type();
    assert_eq!(MongoDbType::Long, mongo_type);

    let nt = model.assert_has_scalar_field("b").assert_native_type();
    let mongo_type: MongoDbType = nt.deserialize_native_type();
    assert_eq!(MongoDbType::Long, mongo_type);
}
