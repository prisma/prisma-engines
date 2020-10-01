use crate::common::parse;

#[test]
fn test_must_not_render_relation_fields_with_many_to_many() {
    let input = r#"model Post {
  id   Int    @id @default(autoincrement())
  User User[]
}

model User {
  id   Int    @id @default(autoincrement())
  Post Post[]
}
"#;

    let expected = input;

    let dml = parse(input);
    println!("{:?}", dml);
    let rendered = datamodel::render_datamodel_to_string(&dml).unwrap();

    print!("{}", rendered);

    assert_eq!(rendered, expected);
}

#[test]
fn test_exclude_default_relation_names_from_rendering() {
    let input = r#"
        model Todo {
            id     Int  @id
            userId Int
            user   User @relation("TodoToUser", fields: [userId], references: [id])
        }

        model User {
            id Int @id
            todo Todo @relation("TodoToUser")
        }
    "#;

    let expected = r#"model Todo {
  id     Int  @id
  userId Int
  user   User @relation(fields: [userId], references: [id])
}

model User {
  id   Int  @id
  todo Todo
}
"#;

    let dml = parse(input);
    let rendered = datamodel::render_datamodel_to_string(&dml).unwrap();

    print!("{}", rendered);

    assert_eq!(rendered, expected);
}

#[test]
fn test_render_relation_name_on_self_relations() {
    let input = r#"model Category {
  createdAt  DateTime
  id         String     @id
  name       String
  updatedAt  DateTime
  Category_A Category[] @relation("CategoryToCategory")
  Category_B Category[] @relation("CategoryToCategory")
}
"#;

    let dml = datamodel::parse_datamodel(input).unwrap();
    let rendered = datamodel::render_datamodel_to_string(&dml).unwrap();

    print!("{}", rendered);

    assert_eq!(rendered, input);
}
