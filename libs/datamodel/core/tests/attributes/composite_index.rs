use expect_test::expect;
use indoc::indoc;

use crate::{common::*, with_header};

#[test]
fn simple_composite_index() {
    let schema = indoc! {r#"
        type A {
          field String
        }

        model B {
          id Int @id @map("_id")
          a  A

          @@index([a.field])
        }
    "#};

    let datamodel = parse(&with_header(schema, crate::Provider::Mongo, &[]));

    datamodel.assert_has_model("B").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("B_a_field_idx".to_string()),
        fields: vec![IndexField::new_in_path(&[("a", None), ("field", Some("A"))])],
        tpe: IndexType::Normal,
        algorithm: None,
        defined_on_field: false,
        clustered: None,
    });
}

#[test]
fn simple_composite_unique() {
    let schema = indoc! {r#"
        type A {
          field String
        }

        model B {
          id Int @id @map("_id")
          a  A

          @@unique([a.field])
        }
    "#};

    let datamodel = parse(&with_header(schema, crate::Provider::Mongo, &[]));

    datamodel.assert_has_model("B").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("B_a_field_key".to_string()),
        fields: vec![IndexField::new_in_path(&[("a", None), ("field", Some("A"))])],
        tpe: IndexType::Unique,
        algorithm: None,
        defined_on_field: false,
        clustered: None,
    });
}

#[test]
fn composite_unique_with_normal_unique() {
    let schema = indoc! {r#"
        type Address {
          street String
          number Int
        }

        model User {
          id      Int     @id @map("_id")
          val     Int     @unique
          address Address

          @@unique([address.number])
        }
    "#};

    let datamodel = parse(&with_header(schema, crate::Provider::Mongo, &[]));

    datamodel
        .assert_has_model("User")
        .assert_has_index(IndexDefinition {
            name: None,
            db_name: Some("User_val_key".to_string()),
            fields: vec![IndexField::new_in_model("val")],
            tpe: IndexType::Unique,
            algorithm: None,
            defined_on_field: true,
            clustered: None,
        })
        .assert_has_index(IndexDefinition {
            name: None,
            db_name: Some("User_address_number_key".to_string()),
            fields: vec![IndexField::new_in_path(&[
                ("address", None),
                ("number", Some("Address")),
            ])],
            tpe: IndexType::Unique,
            algorithm: None,
            defined_on_field: false,
            clustered: None,
        });
}

#[test]
fn simple_composite_fulltext() {
    let schema = indoc! {r#"
        type A {
          field String
        }

        model B {
          id Int @id @map("_id")
          a  A

          @@fulltext([a.field])
        }
    "#};

    let datamodel = parse(&with_header(schema, crate::Provider::Mongo, &["fullTextIndex"]));

    datamodel.assert_has_model("B").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("B_a_field_idx".to_string()),
        fields: vec![IndexField::new_in_path(&[("a", None), ("field", Some("A"))])],
        tpe: IndexType::Fulltext,
        algorithm: None,
        defined_on_field: false,
        clustered: None,
    });
}

#[test]
fn composite_index_with_default() {
    let schema = indoc! {r#"
        type A {
          field String @default("meow")
        }

        model B {
          id Int @id @map("_id")
          a  A

          @@index([a.field])
        }
    "#};

    let datamodel = parse(&with_header(schema, crate::Provider::Mongo, &[]));

    datamodel.assert_has_model("B").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("B_a_field_idx".to_string()),
        fields: vec![IndexField::new_in_path(&[("a", None), ("field", Some("A"))])],
        tpe: IndexType::Normal,
        algorithm: None,
        defined_on_field: false,
        clustered: None,
    });
}

#[test]
fn composite_index_with_map() {
    let schema = indoc! {r#"
        type A {
          field String @map("meow")
        }

        model B {
          id Int @id @map("_id")
          a  A

          @@index([a.field])
        }
    "#};

    let datamodel = parse(&with_header(schema, crate::Provider::Mongo, &[]));

    datamodel.assert_has_model("B").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("B_a_meow_idx".to_string()),
        fields: vec![IndexField::new_in_path(&[("a", None), ("field", Some("A"))])],
        tpe: IndexType::Normal,
        algorithm: None,
        defined_on_field: false,
        clustered: None,
    });
}

#[test]
fn composite_index_with_sort() {
    let schema = indoc! {r#"
        type A {
          field String
        }

        model B {
          id Int @id @map("_id")
          a  A

          @@index([a.field(sort: Desc)])
        }
    "#};

    let datamodel = parse(&with_header(schema, crate::Provider::Mongo, &[]));

    let mut field = IndexField::new_in_path(&[("a", None), ("field", Some("A"))]);
    field.sort_order = Some(SortOrder::Desc);

    datamodel.assert_has_model("B").assert_has_index(IndexDefinition {
        name: None,
        db_name: Some("B_a_field_idx".to_string()),
        fields: vec![field],
        tpe: IndexType::Normal,
        algorithm: None,
        defined_on_field: false,
        clustered: None,
    });
}

#[test]
fn reformat() {
    let schema = indoc! {r#"
        type A {
          field String
          dield String
          gield String
        }

        model B {
          id Int @id @map("_id")
          a  A

          @@index([a.field])
          @@unique([a.dield])
          @@fulltext([a.gield])
        }
    "#};

    let datamodel = with_header(schema, crate::Provider::Mongo, &["fullTextIndex"]);
    let result = datamodel::reformat(&datamodel, 2).unwrap_or_else(|| datamodel.to_owned());

    let expected = expect![[r#"
        datasource test {
          provider = "mongodb"
          url      = "mongo://..."
        }

        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["fullTextIndex"]
        }

        type A {
          field String
          dield String
          gield String
        }

        model B {
          id Int @id @map("_id")
          a  A

          @@unique([a.dield])
          @@index([a.field])
          @@fulltext([a.gield])
        }
    "#]];

    expected.assert_eq(&result);
}

#[test]
fn should_not_work_outside_mongo() {
    let schema = indoc! {r#"
        model B {
          id Int @id @map("_id")
          a  Int

          @@index([a.field])
        }
    "#};

    let dml = with_header(schema, crate::Provider::Postgres, &[]);
    let error = parse_unwrap_err(&dml);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating model "B": The index definition refers to the unknown fields: a.field.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  [1;91m@@index([a.field])[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn should_not_work_outside_mongo_2() {
    let schema = indoc! {r#"
        model C {
          id Int @id @map("_id")
          as A[]
        }

        model A {
          id    Int @id @map("_id")
          field Int
          c     C   @relation(fields: [a.field], references: [id])
        }
    "#};

    let dml = with_header(schema, crate::Provider::Postgres, &[]);
    let error = parse_unwrap_err(&dml);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: The argument fields must refer only to existing fields. The following fields do not exist in this model: a.field[0m
          [1;94m-->[0m  [4mschema.prisma:19[0m
        [1;94m   | [0m
        [1;94m18 | [0m  field Int
        [1;94m19 | [0m  c     C   @relation(fields: [1;91m[a.field][0m, references: [id])
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn a_bonkers_definition_1() {
    let schema = indoc! {r#"
        type A {
          field String
        }

        model B {
          id Int @id @map("_id")
          a  A

          @@index([.field])
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let error = parse_unwrap_err(&dml);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: This line is not a valid field or attribute definition.[0m
          [1;94m-->[0m  [4mschema.prisma:19[0m
        [1;94m   | [0m
        [1;94m18 | [0m
        [1;94m19 | [0m  [1;91m@@index([.field])[0m
        [1;94m20 | [0m}
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn a_bonkers_definition_2() {
    let schema = indoc! {r#"
        type A {
          field String
        }

        model B {
          id Int @id @map("_id")
          a  A

          @@index([a.])
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let error = parse_unwrap_err(&dml);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating model "B": The index definition refers to the unknown fields:  in type A.[0m
          [1;94m-->[0m  [4mschema.prisma:19[0m
        [1;94m   | [0m
        [1;94m18 | [0m
        [1;94m19 | [0m  [1;91m@@index([a.])[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn a_bonkers_definition_3() {
    let schema = indoc! {r#"
        type A {
          field String
        }

        model B {
          id Int @id @map("_id")
          a  A

          @@index([.])
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let error = parse_unwrap_err(&dml);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: This line is not a valid field or attribute definition.[0m
          [1;94m-->[0m  [4mschema.prisma:19[0m
        [1;94m   | [0m
        [1;94m18 | [0m
        [1;94m19 | [0m  [1;91m@@index([.])[0m
        [1;94m20 | [0m}
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn a_bonkers_definition_4() {
    let schema = indoc! {r#"
        type A {
          field String
        }

        model B {
          id Int @id @map("_id")
          a  A

          @@index([....])
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let error = parse_unwrap_err(&dml);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: This line is not a valid field or attribute definition.[0m
          [1;94m-->[0m  [4mschema.prisma:19[0m
        [1;94m   | [0m
        [1;94m18 | [0m
        [1;94m19 | [0m  [1;91m@@index([....])[0m
        [1;94m20 | [0m}
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn a_bonkers_definition_5() {
    let schema = indoc! {r#"
        type A {
          field String
        }

        model B {
          id Int @id @map("_id")
          a  A

          @@index([a .field])
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let error = parse_unwrap_err(&dml);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: This line is not a valid field or attribute definition.[0m
          [1;94m-->[0m  [4mschema.prisma:19[0m
        [1;94m   | [0m
        [1;94m18 | [0m
        [1;94m19 | [0m  [1;91m@@index([a .field])[0m
        [1;94m20 | [0m}
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn a_bonkers_definition_6() {
    let schema = indoc! {r#"
        type A {
          field String
        }

        model B {
          id Int @id @map("_id")
          a  A

          @@index([a something .field])
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let error = parse_unwrap_err(&dml);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: This line is not a valid field or attribute definition.[0m
          [1;94m-->[0m  [4mschema.prisma:19[0m
        [1;94m   | [0m
        [1;94m18 | [0m
        [1;94m19 | [0m  [1;91m@@index([a something .field])[0m
        [1;94m20 | [0m}
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn id_cannot_use_composite_fields() {
    let schema = indoc! {r#"
        type A {
          id Int @map("_id")
        }

        model B {
          a  A

          @@id([a.id])
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let error = parse_unwrap_err(&dml);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating model "B": The multi field id declaration refers to the unknown fields a.id.[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m
        [1;94m18 | [0m  @@id([1;91m[a.id][0m)
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn relation_cannot_use_composite_fields() {
    let schema = indoc! {r#"
        type A {
          field Int
        }

        model C {
          id Int @id @map("_id")
          as A[]
        }

        model B {
          id Int @id @map("_id")
          a  A
          c  C   @relation(fields: [a.field], references: [id])
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let error = parse_unwrap_err(&dml);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: The argument fields must refer only to existing fields. The following fields do not exist in this model: a.field[0m
          [1;94m-->[0m  [4mschema.prisma:23[0m
        [1;94m   | [0m
        [1;94m22 | [0m  a  A
        [1;94m23 | [0m  c  C   @relation(fields: [1;91m[a.field][0m, references: [id])
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn pointing_to_a_non_existing_type() {
    let schema = indoc! {r#"
        type A {
          field String
        }

        model B {
          id Int @id @map("_id")
          a  C

          @@index([a.field])
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let error = parse_unwrap_err(&dml);

    let expected = expect![[r#"
        [1;91merror[0m: [1mType "C" is neither a built-in type, nor refers to another model, custom type, or enum.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  id Int @id @map("_id")
        [1;94m17 | [0m  a  [1;91mC[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn index_to_a_missing_field_in_a_composite_type() {
    let schema = indoc! {r#"
        type A {
          field String
        }

        model B {
          id Int @id @map("_id")
          a  A

          @@index([a.cat])
          @@unique([a.cat])
          @@fulltext([a.cat])
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let error = parse_unwrap_err(&dml);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating model "B": The index definition refers to the unknown fields: cat in type A.[0m
          [1;94m-->[0m  [4mschema.prisma:19[0m
        [1;94m   | [0m
        [1;94m18 | [0m
        [1;94m19 | [0m  [1;91m@@index([a.cat])[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating model "B": The unique index definition refers to the unknown fields: cat in type A.[0m
          [1;94m-->[0m  [4mschema.prisma:20[0m
        [1;94m   | [0m
        [1;94m19 | [0m  @@index([a.cat])
        [1;94m20 | [0m  [1;91m@@unique([a.cat])[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating model "B": The index definition refers to the unknown fields: cat in type A.[0m
          [1;94m-->[0m  [4mschema.prisma:21[0m
        [1;94m   | [0m
        [1;94m20 | [0m  @@unique([a.cat])
        [1;94m21 | [0m  [1;91m@@fulltext([a.cat])[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn index_to_a_missing_composite_field() {
    let schema = indoc! {r#"
        type A {
          field String
        }

        model B {
          id Int @id @map("_id")
          a  A

          @@index([b.field])
          @@unique([b.field])
          @@fulltext([b.field])
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let error = parse_unwrap_err(&dml);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating model "B": The index definition refers to the unknown fields: b.[0m
          [1;94m-->[0m  [4mschema.prisma:19[0m
        [1;94m   | [0m
        [1;94m18 | [0m
        [1;94m19 | [0m  [1;91m@@index([b.field])[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating model "B": The unique index definition refers to the unknown fields: b.[0m
          [1;94m-->[0m  [4mschema.prisma:20[0m
        [1;94m   | [0m
        [1;94m19 | [0m  @@index([b.field])
        [1;94m20 | [0m  [1;91m@@unique([b.field])[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating model "B": The index definition refers to the unknown fields: b.[0m
          [1;94m-->[0m  [4mschema.prisma:21[0m
        [1;94m   | [0m
        [1;94m20 | [0m  @@unique([b.field])
        [1;94m21 | [0m  [1;91m@@fulltext([b.field])[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn non_composite_field_in_the_path() {
    let schema = indoc! {r#"
        type A {
          field String
        }

        model B {
          id Int @id @map("_id")
          a  A
          b  Int

          @@index([b.field, a.field])
          @@unique([b.field, a.field])
          @@fulltext([b.field, a.field])
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let error = parse_unwrap_err(&dml);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating model "B": The index definition refers to the unknown fields: b.field.[0m
          [1;94m-->[0m  [4mschema.prisma:20[0m
        [1;94m   | [0m
        [1;94m19 | [0m
        [1;94m20 | [0m  [1;91m@@index([b.field, a.field])[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating model "B": The unique index definition refers to the unknown fields: b.field.[0m
          [1;94m-->[0m  [4mschema.prisma:21[0m
        [1;94m   | [0m
        [1;94m20 | [0m  @@index([b.field, a.field])
        [1;94m21 | [0m  [1;91m@@unique([b.field, a.field])[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating model "B": The index definition refers to the unknown fields: b.field.[0m
          [1;94m-->[0m  [4mschema.prisma:22[0m
        [1;94m   | [0m
        [1;94m21 | [0m  @@unique([b.field, a.field])
        [1;94m22 | [0m  [1;91m@@fulltext([b.field, a.field])[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn non_composite_field_in_the_middle_of_the_path() {
    let schema = indoc! {r#"
        type A {
          field String
        }

        type C {
          a    A
          bonk String
        }

        model B {
          id Int @id @map("_id")
          c  C

          @@index([c.bonk.field])
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let error = parse_unwrap_err(&dml);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating model "B": The index definition refers to the unknown fields: c.bonk.field.[0m
          [1;94m-->[0m  [4mschema.prisma:24[0m
        [1;94m   | [0m
        [1;94m23 | [0m
        [1;94m24 | [0m  [1;91m@@index([c.bonk.field])[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}
