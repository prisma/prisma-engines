use crate::common::parse;
use datamodel::Datamodel;

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
}"#;

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
}"#;

    let dml = datamodel::parse_datamodel(input).unwrap();
    let rendered = datamodel::render_datamodel_to_string(&dml).unwrap();

    print!("{}", rendered);

    assert_eq!(rendered, input);
}

#[test]
fn experimental_features_roundtrip() {
    let input = r#"generator client {
  provider             = "prisma-client-js"
  experimentalFeatures = ["connectOrCreate", "transactionApi"]
}

datasource db {
  provider = "postgresql"
  url      = "postgresql://test"
}"#;

    let dml = datamodel::parse_configuration(input).unwrap();
    let rendered = datamodel::render_datamodel_and_config_to_string(&Datamodel::new(), &dml).unwrap();

    print!("{}", rendered);
    assert_eq!(input, rendered);
}
