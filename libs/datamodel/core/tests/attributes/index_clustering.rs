use crate::{common::*, with_header, Provider};

#[test]
fn non_boolean_clustering() {
    let dml = indoc! {r#"
        model A {
          id Int @id(clustered: meow)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mExpected a boolean value, but received literal value `meow`.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mmodel A {
        [1;94m12 | [0m  id Int @id(clustered: [1;91mmeow[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn clustered_index_works_on_sql_server() {
    let dml = indoc! {r#"
        model A {
          id Int @id(clustered: false)
          a  Int

          @@index([a], clustered: true)
        }
    "#};

    let schema = with_header(dml, Provider::SqlServer, &[]);
    let schema = parse(&schema);

    schema.assert_has_model("A").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_idx".to_string()),
        fields: vec![IndexField::new_in_model("a")],
        tpe: IndexType::Normal,
        defined_on_field: false,
        algorithm: None,
        clustered: Some(true),
    });
}

#[test]
fn clustered_unique_index_works_on_sql_server() {
    let dml = indoc! {r#"
        model A {
          id Int @id(clustered: false)
          a  Int @unique(clustered: true)
        }
    "#};

    let schema = with_header(dml, Provider::SqlServer, &[]);
    let schema = parse(&schema);
    let model = schema.assert_has_model("A");

    model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_key".to_string()),
        fields: vec![IndexField::new_in_model("a")],
        tpe: IndexType::Unique,
        clustered: Some(true),
        defined_on_field: true,
        algorithm: None,
    });
}

#[test]
fn clustered_compound_unique_index_works_on_sql_server() {
    let dml = indoc! {r#"
        model A {
          id Int @id(clustered: false)
          a  Int
          b  Int

          @@unique([a, b], clustered: true)
        }
    "#};

    let schema = with_header(dml, Provider::SqlServer, &[]);
    let schema = parse(&schema);
    let model = schema.assert_has_model("A");

    model.assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("A_a_b_key".to_string()),
        fields: vec![IndexField::new_in_model("a"), IndexField::new_in_model("b")],
        tpe: IndexType::Unique,
        clustered: Some(true),
        defined_on_field: false,
        algorithm: None,
    });
}

#[test]
fn non_clustered_id_works_on_sql_server() {
    let dml = indoc! {r#"
        model A {
          id Int @id(clustered: false)
        }
    "#};

    let schema = with_header(dml, Provider::SqlServer, &[]);
    let schema = parse(&schema);
    let model = schema.assert_has_model("A");

    model.assert_has_pk(PrimaryKeyDefinition {
        name: None,
        db_name: Some("A_pkey".to_string()),
        fields: vec![PrimaryKeyField::new("id")],
        defined_on_field: true,
        clustered: Some(false),
    });
}

#[test]
fn non_clustered_compound_id_works_on_sql_server() {
    let dml = indoc! {r#"
        model A {
          left  Int
          right Int

          @@id([left, right], clustered: false)
        }
    "#};

    let schema = with_header(dml, Provider::SqlServer, &[]);
    let schema = parse(&schema);
    let model = schema.assert_has_model("A");

    model.assert_has_pk(PrimaryKeyDefinition {
        name: None,
        db_name: Some("A_pkey".to_string()),
        fields: vec![PrimaryKeyField::new("left"), PrimaryKeyField::new("right")],
        defined_on_field: false,
        clustered: Some(false),
    });
}

#[test]
fn clustered_index_allowed_only_in_sql_server() {
    let dml = indoc! {r#"
        model A {
          id Int @id @map("_id")
          a  Int

          @@index([a], clustered: true)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": Defining clustering is not supported in the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a], clustered: true)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn clustered_unique_allowed_only_in_sql_server() {
    let dml = indoc! {r#"
        model A {
          id Int @id @map("_id")
          a  Int @unique(clustered: true)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": Defining clustering is not supported in the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id Int @id @map("_id")
        [1;94m13 | [0m  a  Int [1;91m@unique(clustered: true)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn clustered_compound_unique_allowed_only_in_sql_server() {
    let dml = indoc! {r#"
        model A {
          id Int @id @map("_id")
          a  Int
          b  Int

          @@unique([a, b], clustered: true)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@unique": Defining clustering is not supported in the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  [1;91m@@unique([a, b], clustered: true)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn non_clustered_id_allowed_only_in_sql_server() {
    let dml = indoc! {r#"
        model A {
          id Int @id(clustered: false) @map("_id")
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@id": Defining clustering is not supported in the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mmodel A {
        [1;94m12 | [0m  id Int [1;91m@id(clustered: false)[0m @map("_id")
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn non_clustered_compound_id_allowed_only_in_sql_server() {
    let dml = indoc! {r#"
        model A {
          left  Int
          right Int

          @@id([left, right], clustered: false)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@id": Defining clustering is not supported in the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@id([left, right], clustered: false)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn id_and_index_clustering_together_not_allowed() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a Int

          @@index([a], clustered: true)
        }
    "#};

    let schema = with_header(dml, Provider::SqlServer, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@id": A model can only hold one clustered index or id.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mmodel A {
        [1;94m12 | [0m  id Int [1;91m@id[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@@index": A model can only hold one clustered index or key.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a], clustered: true)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn id_and_unique_clustering_together_not_allowed() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          a  Int @unique(clustered: true)
        }
    "#};

    let schema = with_header(dml, Provider::SqlServer, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@id": A model can only hold one clustered index or id.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mmodel A {
        [1;94m12 | [0m  id Int [1;91m@id[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@unique": A model can only hold one clustered index or key.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id Int @id
        [1;94m13 | [0m  a  Int [1;91m@unique(clustered: true)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn do_not_render_id_default_clustering() {
    let input = indoc! {r#"
        model User {
          id Int @id(clustered: true)
        }
    "#};

    let expected = expect![[r#"
        model User {
          id Int @id
        }
    "#]];

    let schema = with_header(input, Provider::SqlServer, &[]);
    let dml = datamodel::parse_datamodel(&schema).unwrap().subject;
    let rendered = datamodel::render_datamodel_to_string(&dml, None);

    expected.assert_eq(&rendered);
}

#[test]
fn render_id_non_default_clustering() {
    let input = indoc! {r#"
        model User {
          id Int @id(clustered: false)
        }
    "#};

    let expected = expect![[r#"
        model User {
          id Int @id(clustered: false)
        }
    "#]];

    let schema = with_header(input, Provider::SqlServer, &[]);
    let (config, dml) = datamodel::parse_schema(&schema).unwrap();
    let rendered = datamodel::render_datamodel_to_string(&dml, Some(&config));

    expected.assert_eq(&rendered);
}

#[test]
fn do_not_render_compound_id_default_clustering() {
    let input = indoc! {r#"
        model User {
          a Int
          b Int

          @@id([a, b], clustered: true)
        }
    "#};

    let expected = expect![[r#"
        model User {
          a Int
          b Int

          @@id([a, b])
        }
    "#]];

    let schema = with_header(input, Provider::SqlServer, &[]);
    let (config, dml) = datamodel::parse_schema(&schema).unwrap();
    let rendered = datamodel::render_datamodel_to_string(&dml, Some(&config));

    expected.assert_eq(&rendered);
}

#[test]
fn render_compound_id_default_clustering() {
    let input = indoc! {r#"
        model User {
          a Int
          b Int

          @@id([a, b], clustered: false)
        }
    "#};

    let expected = expect![[r#"
        model User {
          a Int
          b Int

          @@id([a, b], clustered: false)
        }
    "#]];

    let schema = with_header(input, Provider::SqlServer, &[]);
    let (config, dml) = datamodel::parse_schema(&schema).unwrap();
    let rendered = datamodel::render_datamodel_to_string(&dml, Some(&config));

    expected.assert_eq(&rendered);
}

#[test]
fn do_not_render_index_default_clustering() {
    let input = indoc! {r#"
        model User {
          id Int @id
          a  Int

          @@index([a], clustered: false)
        }
    "#};

    let expected = expect![[r#"
        model User {
          id Int @id
          a  Int

          @@index([a])
        }
    "#]];

    let schema = with_header(input, Provider::SqlServer, &[]);
    let (config, dml) = datamodel::parse_schema(&schema).unwrap();
    let rendered = datamodel::render_datamodel_to_string(&dml, Some(&config));

    expected.assert_eq(&rendered);
}

#[test]
fn render_index_non_default_clustering() {
    let input = indoc! {r#"
        model User {
          id Int @id(clustered: false)
          a  Int

          @@index([a], clustered: true)
        }
    "#};

    let expected = expect![[r#"
        model User {
          id Int @id(clustered: false)
          a  Int

          @@index([a], clustered: true)
        }
    "#]];

    let schema = with_header(input, Provider::SqlServer, &[]);
    let (config, dml) = datamodel::parse_schema(&schema).unwrap();
    let rendered = datamodel::render_datamodel_to_string(&dml, Some(&config));

    expected.assert_eq(&rendered);
}

#[test]
fn do_not_render_unique_default_clustering() {
    let input = indoc! {r#"
        model User {
          id Int @id
          a  Int @unique(clustered: false)
        }
    "#};

    let expected = expect![[r#"
        model User {
          id Int @id
          a  Int @unique
        }
    "#]];

    let schema = with_header(input, Provider::SqlServer, &[]);
    let (config, dml) = datamodel::parse_schema(&schema).unwrap();
    let rendered = datamodel::render_datamodel_to_string(&dml, Some(&config));

    expected.assert_eq(&rendered);
}

#[test]
fn render_unique_non_default_clustering() {
    let input = indoc! {r#"
        model User {
          id Int @id(clustered: false)
          a  Int @unique(clustered: true)
        }
    "#};

    let expected = expect![[r#"
        model User {
          id Int @id(clustered: false)
          a  Int @unique(clustered: true)
        }
    "#]];

    let schema = with_header(input, Provider::SqlServer, &[]);
    let (config, dml) = datamodel::parse_schema(&schema).unwrap();
    let rendered = datamodel::render_datamodel_to_string(&dml, Some(&config));

    expected.assert_eq(&rendered);
}

#[test]
fn do_not_render_compound_unique_default_clustering() {
    let input = indoc! {r#"
        model User {
          id Int @id
          a  Int
          b  Int

          @@unique([a, b], clustered: false)
        }
    "#};

    let expected = expect![[r#"
        model User {
          id Int @id
          a  Int
          b  Int

          @@unique([a, b])
        }
    "#]];

    let schema = with_header(input, Provider::SqlServer, &[]);
    let (config, dml) = datamodel::parse_schema(&schema).unwrap();
    let rendered = datamodel::render_datamodel_to_string(&dml, Some(&config));

    expected.assert_eq(&rendered);
}

#[test]
fn render_compound_unique_non_default_clustering() {
    let input = indoc! {r#"
        model User {
          id Int @id(clustered: false)
          a  Int
          b  Int

          @@unique([a, b], clustered: true)
        }
    "#};

    let expected = expect![[r#"
        model User {
          id Int @id(clustered: false)
          a  Int
          b  Int

          @@unique([a, b], clustered: true)
        }
    "#]];

    let schema = with_header(input, Provider::SqlServer, &[]);
    let (config, dml) = datamodel::parse_schema(&schema).unwrap();
    let rendered = datamodel::render_datamodel_to_string(&dml, Some(&config));

    expected.assert_eq(&rendered);
}
