use crate::common::*;

#[test]
fn disallow_ignore_missing_from_model_without_fields() {
    let dml = r#"
    model ModelNoFields {
    }
    "#;

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "ModelNoFields": Each model must have at least one unique criteria that has only required fields. Either mark a single field with `@id`, `@unique` or add a multi field criterion with `@@id([])` or `@@unique([])` to the model.[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0m
        [1;94m 2 | [0m    [1;91mmodel ModelNoFields {[0m
        [1;94m 3 | [0m    }
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn disallow_ignore_missing_from_model_without_id() {
    let dml = indoc! {r#"
        model ModelNoId {
          text String
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "ModelNoId": Each model must have at least one unique criteria that has only required fields. Either mark a single field with `@id`, `@unique` or add a multi field criterion with `@@id([])` or `@@unique([])` to the model.[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91mmodel ModelNoId {[0m
        [1;94m 2 | [0m  text String
        [1;94m 3 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn disallow_ignore_missing_from_model_with_optional_id() {
    let dml = indoc! {r#"
        model ModelOptionalId {
          text String? @id
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@id": Fields that are marked as id must be required.[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0mmodel ModelOptionalId {
        [1;94m 2 | [0m  text String? [1;91m@id[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn disallow_ignore_missing_from_model_with_unsupported_id() {
    let dml = indoc! {r#"
        model ModelUnsupportedId {
          text Unsupported("something") @id
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "ModelUnsupportedId": Each model must have at least one unique criteria that has only required fields. Either mark a single field with `@id`, `@unique` or add a multi field criterion with `@@id([])` or `@@unique([])` to the model. The following unique criterias were not considered as they contain fields that are not required:
        - text[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91mmodel ModelUnsupportedId {[0m
        [1;94m 2 | [0m  text Unsupported("something") @id
        [1;94m 3 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn disallow_ignore_missing_from_model_with_compound_unsupported_id() {
    let dml = indoc! {r#"
        model ModelCompoundUnsupportedId {
          text Unsupported("something")
          int  Int

          @@id([text, int])
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "ModelCompoundUnsupportedId": Each model must have at least one unique criteria that has only required fields. Either mark a single field with `@id`, `@unique` or add a multi field criterion with `@@id([])` or `@@unique([])` to the model. The following unique criterias were not considered as they contain fields that are not required:
        - text, int[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91mmodel ModelCompoundUnsupportedId {[0m
        [1;94m 2 | [0m  text Unsupported("something")
        [1;94m 3 | [0m  int  Int
        [1;94m 4 | [0m
        [1;94m 5 | [0m  @@id([text, int])
        [1;94m 6 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn disallow_ignore_on_models_with_relations_pointing_to_them() {
    let dml = indoc! {r#"
        model ModelValidC {
          id Int @id
          d  Int
          rel_d  ModelValidD @relation(fields: d, references: id) //ignore here is missing
        }

        model ModelValidD {
          id Int @id
          rel_c  ModelValidC[]

          @@ignore
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@ignore": The relation field `rel_d` on Model `ModelValidC` must specify the `@ignore` attribute, because the model ModelValidD it is pointing to is marked ignored.[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  d  Int
        [1;94m 4 | [0m  [1;91mrel_d  ModelValidD @relation(fields: d, references: id) //ignore here is missing[0m
        [1;94m 5 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn disallow_ignore_on_models_with_back_relations_pointing_to_them() {
    let dml = indoc! {r#"
        model ModelValidA {
          id Int @id
          b  Int
          rel_b  ModelValidB @relation(fields: b, references: id)

          @@ignore
        }

        model ModelValidB {
          id Int @id
          rel_a  ModelValidA[] //ignore is missing here
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@ignore": The relation field `rel_a` on Model `ModelValidB` must specify the `@ignore` attribute, because the model ModelValidA it is pointing to is marked ignored.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m  id Int @id
        [1;94m11 | [0m  [1;91mrel_a  ModelValidA[] //ignore is missing here[0m
        [1;94m12 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn disallow_ignore_on_unsupported() {
    let dml = indoc! {r#"
        model ModelValidA {
          id Int @id
          b  Unsupported("something") @ignore
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@ignore": Fields of type `Unsupported` cannot take an `@ignore` attribute. They are already treated as ignored by the client due to their type.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  b  Unsupported("something") [1;91m@ignore[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn disallow_ignore_on_ignored_model() {
    let dml = indoc! {r#"
        model ModelValidA {
          id Int @id
          b  String @ignore

          @@ignore
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@ignore": Fields on an already ignored Model do not need an `@ignore` annotation.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  [1;91mb  String @ignore[0m
        [1;94m 4 | [0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}
