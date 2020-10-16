use pretty_assertions::assert_eq;

#[test]
fn test_parser_renderer_via_ast() {
    let input = r#"model User {
  id        Int      @id
  createdAt DateTime
  email     String   @unique
  name      String?
  posts     Post[]   @relation(onDelete: CASCADE)
  profile   Profile?

  @@unique([email, name])
  @@unique([name, email])
  @@map("user")
}

model Profile {
  id   Int    @id
  user User
  bio  String

  @@map("profile")
}

model Post {
  id         Int
  createdAt  DateTime
  updatedAt  DateTime
  title      String           @default("Default-Title")
  wasLiked   Boolean          @default(false)
  author     User             @relation("author")
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
  id       Int      @id
  post     Post
  category Category

  @@map("post_to_category")
}

model A {
  id Int @id
  b  B
}

model B {
  id Int @id
  a  A
}

enum CategoryEnum {
  A
  B
  C
}
"#;
    assert_rendered(input, input);
}

#[test]
fn test_parser_renderer_many_to_many_via_ast() {
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
  id    Int      @id
  title String
  tags  String[]
  blog  Blog
}
"#;
    assert_rendered(input, input);
}

#[test]
fn test_parser_renderer_types_via_ast() {
    let input = r#"type ID = Int @id

model Author {
  id      ID
  name    String?
  authors Blog[]  @relation("AuthorToBlogs")
}
"#;

    assert_rendered(input, input);
}

#[test]
fn test_parser_renderer_native_types_via_ast() {
    let input = r#"datasource pg {
  provider = "postgresql"
  url      = "postgresql://"
}

generator js {
  provider        = "prisma-client-js"
  previewFeatures = ["nativeTypes"]
}

model Blog {
  id     Int    @id
  bigInt Int    @pg.BigInt
  foobar String @pg.VarChar(12)
}
"#;

    assert_rendered(input, input);
}

#[test]
fn test_parser_renderer_order_of_field_attributes_via_ast() {
    let input = r#"model Post {
  id        Int      @default(autoincrement()) @id
  published Boolean  @map("_published") @default(false)
  author    User?   @relation(fields: [authorId], references: [id])
  authorId  Int?
}

model User {
  megaField DateTime @map("mega_field") @id @default("10.02.1010") @unique @updatedAt
}

model Test {
  id     Int   @id @map("_id") @default(1) @updatedAt
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
  megaField DateTime @id @unique @default("10.02.1010") @updatedAt @map("mega_field")
}

model Test {
  id     Int  @id @default(1) @updatedAt @map("_id")
  blogId Int? @unique @default(1)
}
"#;

    assert_rendered(input, expected);
}

#[test]
fn test_parser_renderer_order_of_block_attributes_via_ast() {
    let input = r#"model Person {
  firstName   String
  lastName    String
  codeName    String
  yearOfBirth Int
  @@map("blog")
  @@index([yearOfBirth])
  @@unique([codeName, yearOfBirth])
  @@id([firstName, lastName])
}

model Blog {
  id    Int    @default(1)
  name  String
  posts Post[]
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
  @@map("blog")
}

model Blog {
  id    Int    @default(1)
  name  String
  posts Post[]

  @@id([id])
  @@unique([name])
  @@index([id, name])
  @@map("blog")
}
"#;
    assert_rendered(input, expected);
}

#[test]
fn test_parser_renderer_sources_via_ast() {
    let input = r#"datasource pg1 {
  provider = "Postgres"
  url      = "https://localhost/postgres1"
}

model Author {
  id      ID
  name    String?
  authors Blog[]  @relation("AuthorToBlogs")
}
"#;

    assert_eq!(input, input);
}

#[test]
fn test_parser_renderer_sources_and_comments_via_ast() {
    let input = r#"/// Super cool postgres source.
datasource pg1 {
  provider = "postgres"
  url      = "https://localhost/postgres1"
}

/// My author model.
model Author {
  id        Int      @id
  /// Name of the author.
  name      String?
  createdAt DateTime @default(now())
}
"#;

    assert_rendered(input, input);
}

#[test]
fn test_parser_renderer_with_tabs() {
    let input = r#"/// Super cool postgres source.
datasource\tpg1\t{
\tprovider\t=\t\t"postgres"
\turl\t=\t"https://localhost/postgres1"
}
\t
///\tMy author\tmodel.
model\tAuthor\t{
\tid\tInt\t@id
\t/// Name of the author.
\t\tname\tString?
\tcreatedAt\tDateTime\t@default(now())
}"#;

    let expected = r#"/// Super cool postgres source.
datasource pg1 {
  provider = "postgres"
  url      = "https://localhost/postgres1"
}

/// My author\tmodel.
model Author {
  id        Int      @id
  /// Name of the author.
  name      String?
  createdAt DateTime @default(now())
}
"#;
    // replaces \t placeholder with a real tab
    let tabbed_dm = input.replace("\\t", "\t");
    assert_rendered(&tabbed_dm, &expected.replace("\\t", "\t"));
}

fn assert_rendered(input: &str, expected: &str) {
    let ast = parse_to_ast(&input).expect("failed to parse");
    let rendered = render_schema_ast_to_string(&ast);
    assert_eq!(rendered, expected);
}

fn render_schema_ast_to_string(schema: &datamodel::ast::SchemaAst) -> String {
    let mut writable_string = datamodel::common::WritableString::new();
    let mut renderer = datamodel::ast::renderer::Renderer::new(&mut writable_string, 2);
    renderer.render(schema);
    writable_string.into()
}

fn parse_to_ast(datamodel_string: &str) -> Result<datamodel::ast::SchemaAst, datamodel::diagnostics::Diagnostics> {
    datamodel::ast::parser::parse_schema(datamodel_string)
}
