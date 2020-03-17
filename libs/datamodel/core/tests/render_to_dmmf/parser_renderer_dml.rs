extern crate datamodel;
use pretty_assertions::assert_eq;

// TODO: test `onDelete` back once `prisma migrate` is a thing
const DATAMODEL_STRING: &str = r#"model User {
  id        Int      @id
  createdAt DateTime
  email     String   @unique
  name      String?
  posts     Post[]   @relation("author")
  profile   Profile?

  @@map("user")
  @@unique([email, name])
  @@unique([name, email])
}

model Profile {
  id   Int    @id
  user User   @relation(references: [id])
  bio  String

  @@map("profile")
}

model Post {
  id         Int
  createdAt  DateTime
  updatedAt  DateTime
  title      String           @default("Default-Title")
  wasLiked   Boolean          @default(false)
  author     User             @relation("author", references: [id])
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
  post     Post     @relation(references: [title, createdAt])
  category Category @relation(references: [id])

  @@map("post_to_category")
}

model A {
  id Int @id
  b  B   @relation(references: [id])
}

model B {
  id Int @id
  a  A
}

enum CategoryEnum {
  A
  B
  C
}"#;

#[test]
fn test_parser_renderer_via_dml() {
    let dml = datamodel::parse_datamodel(DATAMODEL_STRING).unwrap();
    let rendered = datamodel::render_datamodel_to_string(&dml).unwrap();

    print!("{}", rendered);

    assert_eq!(DATAMODEL_STRING, rendered);
}

const MANY_TO_MANY_DATAMODEL: &str = r#"model Blog {
  id        Int      @id
  name      String
  viewCount Int
  posts     Post[]
  authors   Author[] @relation("AuthorToBlogs", references: [id])
}

model Author {
  id      Int     @id
  name    String?
  authors Blog[]  @relation("AuthorToBlogs", references: [id])
}

model Post {
  id    Int    @id
  title String
  blog  Blog   @relation(references: [id])
}"#;

#[test]
fn test_parser_renderer_many_to_many_via_dml() {
    let dml = datamodel::parse_datamodel(MANY_TO_MANY_DATAMODEL).unwrap();
    let rendered = datamodel::render_datamodel_to_string(&dml).unwrap();

    print!("{}", rendered);

    assert_eq!(rendered, MANY_TO_MANY_DATAMODEL);
}

const DATAMODEL_STRING_WITH_COMMENTS: &str = r#"// Cool user model
model User {
  id        Int      @id
  // Created at field
  createdAt DateTime
  email     String   @unique
  // Name field.
  // Multi line comment.
  name      String?

  @@map("user")
}"#;

#[test]
fn test_parser_renderer_model_with_comments_via_dml() {
    let dml = datamodel::parse_datamodel(DATAMODEL_STRING_WITH_COMMENTS).unwrap();
    let rendered = datamodel::render_datamodel_to_string(&dml).unwrap();

    print!("{}", rendered);

    assert_eq!(rendered, DATAMODEL_STRING_WITH_COMMENTS);
}
