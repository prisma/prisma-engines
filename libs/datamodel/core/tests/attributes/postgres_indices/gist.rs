use crate::{common::*, with_header, Provider};

#[test]
fn not_allowed_with_unique() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @unique(type: Gist)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNo such argument.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id Int @id
        [1;94m13 | [0m  a  Int @unique([1;91mtype: Gist[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn not_allowed_with_compound_unique() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int
          b  Int

          @@unique([a, b], type: Gist)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNo such argument.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  @@unique([a, b], [1;91mtype: Gist[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn on_mysql() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String

          @@index([a(ops: raw("test_ops"))], type: Gist)
        }
    "#};

    let schema = with_header(dml, Provider::Mysql, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given index type is not supported with the current connector[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@index([a(ops: raw("test_ops"))], [1;91mtype: Gist[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn with_inet() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Inet

          @@index([a(ops: InetOps)], type: Gist)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let mut field = IndexField::new_in_model("a");
    field.operator_class = Some(OperatorClass::InetOps);

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::Gist),
        clustered: None,
    });
}

#[test]
fn with_raw_unsupported() {
    let dml = indoc! {r#"
        model A {
          id Int                @id
          a  Unsupported("box")

          @@index([a(ops: raw("box_ops"))], type: Gist)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let mut field = IndexField::new_in_model("a");
    field.operator_class = Some(OperatorClass::raw("box_ops"));

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::Gist),
        clustered: None,
    });
}

#[test]
fn with_unsupported_no_ops() {
    let dml = indoc! {r#"
        model A {
          id Int                @id
          a  Unsupported("box")

          @@index([a], type: Gist)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let schema = parse(&schema);

    let field = IndexField::new_in_model("a");

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: Some(IndexAlgorithm::Gist),
        clustered: None,
    });
}

#[test]
fn wrong_ops_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.VarChar(255)

          @@index([a(ops: InetOps)], type: Gist)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `InetOps` does not support native type `VarChar` of field `a`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: InetOps)], type: Gist)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn wrong_ops_index_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String @test.Inet

          @@index([a(ops: InetOps)], type: Hash)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `InetOps` is not supported with the `Hash` index type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: InetOps)], type: Hash)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn wrong_ops_no_native_type() {
    let dml = indoc! {r#"
        model A {
          id Int    @id
          a  String

          @@index([a(ops: InetOps)], type: Gist)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = parse_unwrap_err(&schema);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given operator class `InetOps` expects the field `a` to define a valid native type.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a(ops: InetOps)], type: Gist)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}
