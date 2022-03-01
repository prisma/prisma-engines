use crate::common::*;
use expect_test::expect;
use indoc::indoc;

#[test]
fn test_must_not_render_relation_fields_with_many_to_many() {
    let input = indoc! {r#"
        model Post {
          id   Int    @id @default(autoincrement())
          User User[]
        }

        model User {
          id   Int    @id @default(autoincrement())
          Post Post[]
        }
    "#};

    let expected = expect![[r#"
        model Post {
          id   Int    @id @default(autoincrement())
          User User[]
        }

        model User {
          id   Int    @id @default(autoincrement())
          Post Post[]
        }
    "#]];

    let result = datamodel::render_datamodel_to_string(&parse(input), None);
    expected.assert_eq(&result);
}

#[test]
fn test_exclude_default_relation_names_from_rendering() {
    let input = indoc! {r#"
        model Todo {
          id     Int  @id
          userId Int
          user   User @relation("TodoToUser", fields: [userId], references: [id])
        }

        model User {
          id   Int   @id
          todo Todo? @relation("TodoToUser")
        }
    "#};

    let expected = expect![[r#"
        model Todo {
          id     Int  @id
          userId Int  @unique
          user   User @relation(fields: [userId], references: [id])
        }

        model User {
          id   Int   @id
          todo Todo?
        }
    "#]];

    let result = datamodel::render_datamodel_to_string(&parse(input), None);
    expected.assert_eq(&result);
}

#[test]
fn test_render_relation_name_on_self_relations() {
    let input = indoc! {r#"
        model Category {
          createdAt  DateTime
          id         String     @id
          name       String
          updatedAt  DateTime
          Category_A Category[] @relation("CategoryToCategory")
          Category_B Category[] @relation("CategoryToCategory")
        }
    "#};

    let expected = expect![[r#"
        model Category {
          createdAt  DateTime
          id         String     @id
          name       String
          updatedAt  DateTime
          Category_A Category[] @relation("CategoryToCategory")
          Category_B Category[] @relation("CategoryToCategory")
        }
    "#]];

    let result = datamodel::render_datamodel_to_string(&parse(input), None);
    expected.assert_eq(&result);
}
