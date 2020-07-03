use crate::common::parse;

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
  Category_A Category[] @relation("CategoryToCategory", references: [id])
  Category_B Category[] @relation("CategoryToCategory", references: [id])
}
"#;

    let dml = datamodel::parse_datamodel(input).unwrap();
    let rendered = datamodel::render_datamodel_to_string(&dml).unwrap();

    print!("{}", rendered);

    assert_eq!(rendered, input);
}
