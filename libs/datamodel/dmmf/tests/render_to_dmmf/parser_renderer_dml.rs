use crate::common::*;
use datamodel::Datamodel;
use indoc::indoc;
use pretty_assertions::assert_eq;

#[test]
fn test_parser_renderer_via_dml() {
    // TODO: test `onDelete` back once `prisma migrate` is a thing
    let input = indoc! {r#"
        model User {
          id        Int      @id
          createdAt DateTime
          email     String   @unique
          name      String?
          posts     Post[]   @relation("author")
          profile   Profile?

          @@unique([email, name])
          @@unique([name, email])
          @@map("user")
        }

        model Profile {
          id     Int    @id
          bio    String
          userId Int
          user   User   @relation(fields: [userId], references: [id])

          @@map("profile")
        }

        model Post {
          id         Int
          createdAt  DateTime
          updatedAt  DateTime
          title      String           @default("Default-Title")
          wasLiked   Boolean          @default(false)
          authorId   Int
          author     User             @relation("author", fields: [authorId], references: [id])
          published  Boolean          @default(false)
          categories PostToCategory[]

          @@id([title, createdAt])
          @@map("post")
        }

        model Category {
          id    Int              @id
          name  String
          posts PostToCategory[]
          cat   CategoryEnum

          @@map("category")
        }

        model PostToCategory {
          id            Int      @id
          postTitle     String
          postCreatedAt DateTime
          categoryId    Int
          post          Post     @relation(fields: [postTitle, postCreatedAt], references: [title, createdAt])
          category      Category @relation(fields: [categoryId], references: [id])

          @@map("post_to_category")
        }

        model A {
          id  Int @id
          bId Int
          b   B   @relation(fields: [bId], references: [id])
        }

        model B {
          id Int @id
          a  A?
        }

        enum CategoryEnum {
          A
          B
          C
        }
    "#};

    let expected = expect![[r#"
        model User {
          id        Int      @id
          createdAt DateTime
          email     String   @unique
          name      String?
          posts     Post[]   @relation("author")
          profile   Profile?

          @@unique([email, name])
          @@unique([name, email])
          @@map("user")
        }

        model Profile {
          id     Int    @id
          bio    String
          userId Int    @unique
          user   User   @relation(fields: [userId], references: [id])

          @@map("profile")
        }

        model Post {
          id         Int
          createdAt  DateTime
          updatedAt  DateTime
          title      String           @default("Default-Title")
          wasLiked   Boolean          @default(false)
          authorId   Int
          author     User             @relation("author", fields: [authorId], references: [id])
          published  Boolean          @default(false)
          categories PostToCategory[]

          @@id([title, createdAt])
          @@map("post")
        }

        model Category {
          id    Int              @id
          name  String
          posts PostToCategory[]
          cat   CategoryEnum

          @@map("category")
        }

        model PostToCategory {
          id            Int      @id
          postTitle     String
          postCreatedAt DateTime
          categoryId    Int
          post          Post     @relation(fields: [postTitle, postCreatedAt], references: [title, createdAt])
          category      Category @relation(fields: [categoryId], references: [id])

          @@map("post_to_category")
        }

        model A {
          id  Int @id
          bId Int @unique
          b   B   @relation(fields: [bId], references: [id])
        }

        model B {
          id Int @id
          a  A?
        }

        enum CategoryEnum {
          A
          B
          C
        }
    "#]];

    expected.assert_eq(&datamodel::render_datamodel_to_string(&parse(input), None));
}

#[test]
fn test_parser_renderer_order_of_field_attributes_via_dml() {
    let input = r#"model Post {
  id        Int      @default(autoincrement()) @id
  published Boolean  @map("_published") @default(false)
  author    User?   @relation(fields: [authorId], references: [id])
  authorId  Int?
}

model User {
  id Int @id
  megaField DateTime @map("mega_field") @default(now()) @unique @updatedAt
  Post Post[]
}

model Test {
  id     Int   @id @map("_id") @default(1)
  blogId Int?  @unique @default(1)
}
"#;
    let expected = r#"model Post {
  id        Int     @id @default(autoincrement())
  published Boolean @default(false) @map("_published")
  author    User?   @relation(fields: [authorId], references: [id])
  authorId  Int?
}

model User {
  id        Int      @id
  megaField DateTime @unique @default(now()) @updatedAt @map("mega_field")
  Post      Post[]
}

model Test {
  id     Int  @id @default(1) @map("_id")
  blogId Int? @unique @default(1)
}
"#;

    let dml = parse(input);
    let rendered = datamodel::render_datamodel_to_string(&dml, None);

    print!("{}", rendered);

    assert_eq!(expected, rendered);
}

#[test]
fn test_parser_renderer_order_of_block_attributes_via_dml() {
    let input = r#"model Person {
  firstName   String
  lastName    String
  codeName    String
  yearOfBirth Int
  @@map("person")
  @@index([yearOfBirth])
  @@unique([codeName, yearOfBirth])
  @@id([firstName, lastName])
}

model Blog {
  id   Int    @default(1)
  name String
  @@id([id])
  @@index([id, name])
  @@unique([name])
  @@map("blog")
}
"#;

    let expected = r#"model Person {
  firstName   String
  lastName    String
  codeName    String
  yearOfBirth Int

  @@id([firstName, lastName])
  @@unique([codeName, yearOfBirth])
  @@index([yearOfBirth])
  @@map("person")
}

model Blog {
  id   Int    @default(1)
  name String

  @@id([id])
  @@unique([name])
  @@index([id, name])
  @@map("blog")
}
"#;

    let dml = parse(input);
    let rendered = datamodel::render_datamodel_to_string(&dml, None);

    print!("{}", rendered);

    assert_eq!(expected, rendered);
}

#[test]
fn test_parser_renderer_many_to_many_via_dml() {
    let input = r#"model Blog {
  id        Int      @id
  name      String
  viewCount Int
  posts     Post[]
  authors   Author[] @relation("AuthorToBlogs")
}

model Author {
  id      Int     @id
  name    String?
  authors Blog[]  @relation("AuthorToBlogs")
}

model Post {
  id     Int    @id
  title  String
  blogId Int
  blog   Blog   @relation(fields: [blogId], references: [id])
}
"#;

    let dml = parse(input);
    let rendered = datamodel::render_datamodel_to_string(&dml, None);

    print!("{}", rendered);

    assert_eq!(rendered, input);
}

#[test]
fn test_parser_renderer_model_with_comments_via_dml() {
    let input = r#"/// Cool user model
model User {
  id        Int      @id
  /// Created at field
  createdAt DateTime
  email     String   @unique
  /// Name field.
  /// Multi line comment.
  name      String?

  @@map("user")
}
"#;

    let dml = parse(input);
    let rendered = datamodel::render_datamodel_to_string(&dml, None);

    print!("{}", rendered);

    assert_eq!(rendered, input);
}

#[test]
fn test_parser_renderer_native_types_via_dml() {
    // Here BigInt is not rendered because it is the default.
    let input = indoc!(
        r#"
        datasource pg {
          provider = "postgresql"
          url      = "postgresql://"
        }

        model Blog {
          id     Int    @id
          bigInt BigInt
          foobar String @pg.VarChar(12)
        }
        "#
    );

    let dml = parse(input);

    println!("{:?}", dml);

    let config = parse_configuration(input);
    let dml = parse(input);
    let rendered = datamodel::render_datamodel_and_config_to_string(&dml, &config);

    assert_eq!(rendered, input);
}

#[test]
fn test_parser_renderer_native_types_with_default_mappings() {
    let dm1 = indoc!(
        r#"
        datasource pg {
          provider = "postgresql"
          url      = "postgresql://"
        }

        model Blog {
          id     Int    @id
          bigInt BigInt
          defaultString String @pg.Text
          defaultInt Int @pg.Integer
          foobar String @pg.VarChar(12)
          otherFloat Float @pg.Real
        }
        "#
    );

    let dm2 = indoc!(
        r#"
        datasource pg {
          provider = "postgresql"
          url      = "postgresql://"
        }

        model Blog {
          id            Int    @id
          bigInt        BigInt
          defaultString String
          defaultInt    Int
          foobar        String @pg.VarChar(12)
          otherFloat    Float  @pg.Real
        }
        "#
    );

    let config = parse_configuration(dm1);
    let dml = parse(dm1);
    let rendered = datamodel::render_datamodel_and_config_to_string(&dml, &config);

    println!("{}\n\n{}", rendered, dm2);
    assert_eq!(rendered, dm2);
}

#[test]
fn test_parser_renderer_native_types_with_default_mappings_mysql() {
    let dm1 = indoc!(
        r#"
        datasource dolphin {
          provider = "mysql"
          url      = "mysql://"
        }

        model Blog {
          id     Int    @id
          bigInt BigInt
          defaultString String @dolphin.VarChar(191)
          defaultInt Int @dolphin.Int
          foobar String @dolphin.VarChar(12)
          otherFloat Float @dolphin.Float
        }
        "#
    );

    let dm2 = indoc!(
        r#"
        datasource dolphin {
          provider = "mysql"
          url      = "mysql://"
        }

        model Blog {
          id            Int    @id
          bigInt        BigInt
          defaultString String
          defaultInt    Int
          foobar        String @dolphin.VarChar(12)
          otherFloat    Float  @dolphin.Float
        }
        "#
    );

    let config = parse_configuration(dm1);
    let dml = parse(dm1);
    let rendered = datamodel::render_datamodel_and_config_to_string(&dml, &config);

    println!("{}\n\n{}", rendered, dm2);
    assert_eq!(rendered, dm2);
}

#[test]
fn preview_features_roundtrip() {
    // we keep the support for `experimentalFeatures` for backwards compatibility reasons
    let input_with_experimental = r#"generator client {
  provider             = "prisma-client-js"
  experimentalFeatures = ["connectOrCreate", "transactionApi"]
}

datasource db {
  provider = "postgresql"
  url      = "postgresql://test"
}
"#;

    let input_with_preview = r#"generator client {
  provider        = "prisma-client-js"
  previewFeatures = ["connectOrCreate", "transactionApi"]
}

datasource db {
  provider = "postgresql"
  url      = "postgresql://test"
}
"#;

    // check that `experimentalFeatures` is turned into `previewFeatures`.
    {
        let config = parse_configuration(input_with_experimental);
        let rendered = datamodel::render_datamodel_and_config_to_string(&Datamodel::new(), &config);
        assert_eq!(rendered, input_with_preview);
    }

    // check that `previewFeatures` stays as is.
    {
        let config = parse_configuration(input_with_preview);
        let rendered = datamodel::render_datamodel_and_config_to_string(&Datamodel::new(), &config);
        assert_eq!(rendered, input_with_preview);
    }
}

#[test]
fn test_parser_renderer_ignored_via_dml() {
    let input = indoc!(
        r#"
        datasource pg {
          provider = "postgresql"
          url      = "postgresql://"
        }

        model Post {
          id      Int
          user_ip Int
          User    User @relation(fields: [user_ip], references: [ip])

          @@ignore
        }

        model User {
          id   Int    @id @default(autoincrement())
          ip   Int    @unique
          Post Post[] @ignore
        }
        "#
    );

    let config = parse_configuration(input);
    let dml = parse(input);
    println!("{:#?}", dml);
    let rendered = datamodel::render_datamodel_and_config_to_string(&dml, &config);

    assert_eq!(rendered, input);
}
