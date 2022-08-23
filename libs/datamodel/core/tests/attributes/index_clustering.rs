use crate::{common::*, with_header, Provider};

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
    let dml = parse(&schema);
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
