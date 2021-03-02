use std::sync::Arc;

use super::helpers::*;
use query_core::PRISMA_NAMESPACE;
use serial_test::serial;

// Tests in this file run serially because the function `get_query_schema` depends on setting an env var.

// Read

#[test]
#[serial]
fn ignored_fields_should_be_filtered_from_input_and_output_types() {
    let dm = r#"
        datasource pg {
            provider = "postgresql"
            url = "postgresql://"
        }

        model Item {
            id           String @id
            ignored_field String @ignore
        }
    "#;
    let (query_schema, datamodel) = get_query_schema(dm);
    let dmmf = crate::dmmf::render_dmmf(&datamodel, Arc::new(query_schema));
    let output_types = dmmf.schema.output_object_types.get(PRISMA_NAMESPACE).unwrap();
    let input_types = dmmf.schema.input_object_types.get(PRISMA_NAMESPACE).unwrap();

    for o in output_types {
        iterate_output_type_fields(o, &dmmf, &|field, _| assert_ne!(field.name, "ignored_field"))
    }

    for o in input_types {
        iterate_input_type_fields(o, &dmmf, &|_, field, _| assert_ne!(field.name, "ignored_field"))
    }
}

#[test]
#[serial]
fn ignored_compound_indices_should_be_filtered_from_input_and_output_types() {
    let dm = r#"
        datasource pg {
            provider = "postgresql"
            url = "postgresql://"
        }

        model Valid {
          id Int @id
        }

        model Item {
          id Int
          a String @ignore
          b String @ignore
          c String @ignore
          d String @ignore

          @@index([a, b])
          @@unique([c, d])
        }
    "#;
    let (query_schema, datamodel) = get_query_schema(dm);
    let dmmf = crate::dmmf::render_dmmf(&datamodel, Arc::new(query_schema));
    let input_types = dmmf
        .schema
        .input_object_types
        .get(PRISMA_NAMESPACE)
        .expect("prisma namespace should exist");

    for o in input_types {
        iterate_input_type_fields(o, &dmmf, &|_, field, _| {
            assert_ne!(field.name, "a_b", "compound unique 'a_b' should not be present");
            assert_ne!(field.name, "c_d", "compound unique 'c_d' should not be present");
        })
    }
}

#[test]
#[serial]
fn relation_with_ignored_fk_fields_should_be_filtered_from_input_output_types() {
    let dm = r#"
        datasource pg {
            provider = "postgresql"
            url = "postgresql://"
        }
        model Post {
          id                Int                              @id @default(autoincrement())
          /// This type is currently not supported.
          ignored_field  String @ignore @default(dbgenerated("X"))
          user              User                             @relation(fields: [ignored_field], references: [balance])
        }

        model User {
          id            Int                               @id @default(autoincrement())
          /// This type is currently not supported.
          balance       String @ignore  @unique
          post          Post[]
        }
    "#;
    let (query_schema, datamodel) = get_query_schema(dm);
    let dmmf = crate::dmmf::render_dmmf(&datamodel, Arc::new(query_schema));
    let output_types = dmmf.schema.output_object_types.get(PRISMA_NAMESPACE).unwrap();
    let input_types = dmmf.schema.input_object_types.get(PRISMA_NAMESPACE).unwrap();

    for o in output_types {
        iterate_output_type_fields(o, &dmmf, &|field, _| {
            assert_ne!(field.name, "user");
            assert_ne!(field.name, "post");
        })
    }

    for o in input_types {
        iterate_input_type_fields(o, &dmmf, &|_, field, _| {
            assert_ne!(field.name, "user");
            assert_ne!(field.name, "post");
        })
    }
}

#[test]
#[serial]
fn no_find_unique_when_model_only_has_ignored_index_or_compound() {
    let dm = r#"
        datasource pg {
            provider = "postgresql"
            url = "postgresql://"
        }

        model ItemA {
          id                Int
          /// This type is currently not supported.
          ignored_index_a  String @ignore  @id
          ignored_index_c  String @ignore  @unique
          ignored_index_d  String @ignore  @unique @default(dbgenerated("X"))
        }

        model ItemB {
          id                Int
          /// This type is currently not supported.
          ignored_index_a  String @ignore  @id @default(dbgenerated("X"))
        }

        model ItemC {
          id Int
          ignored_index_a String @ignore
          ignored_index_b String @ignore
          ignored_index_c String @ignore
          ignored_index_d String @ignore

          @@index([ignored_index_a, ignored_index_b])
          @@unique([ignored_index_c, ignored_index_d])
        }
    "#;
    let (query_schema, datamodel) = get_query_schema(dm);
    let dmmf = crate::dmmf::render_dmmf(&datamodel, Arc::new(query_schema));
    let query_type = dmmf
        .schema
        .output_object_types
        .get(PRISMA_NAMESPACE)
        .unwrap()
        .iter()
        .find(|o| o.name == "Query")
        .unwrap();
    let field_names: Vec<&str> = query_type.fields.iter().map(|f| f.name.as_str()).collect();

    assert!(!field_names.contains(&"findUniqueItemA"));
    assert!(!field_names.contains(&"findUniqueItemB"));
    assert!(!field_names.contains(&"findUniqueItemC"));
}

// Write

#[test]
#[serial]
fn no_create_or_upsert_should_exist_with_ignored_field_without_default_value() {
    let dm = r#"
        datasource pg {
            provider = "postgresql"
            url = "postgresql://"
        }

        model Item {
          id       Int              @id
          /// This type is currently not supported.
          index_a  String @ignore
        }
    "#;
    let (query_schema, datamodel) = get_query_schema(dm);
    let dmmf = crate::dmmf::render_dmmf(&datamodel, Arc::new(query_schema));
    let mutation_type = dmmf
        .schema
        .output_object_types
        .get(PRISMA_NAMESPACE)
        .unwrap()
        .iter()
        .find(|o| o.name == "Mutation")
        .unwrap();

    let field_names: Vec<&str> = mutation_type.fields.iter().map(|f| f.name.as_str()).collect();

    let ignored_ops: [&str; 3] = ["createOne", "createMany", "upsertOne"];
    ignored_ops.iter().for_each(|op| {
        assert!(
            !field_names.contains(&format!("{}Item", *op).as_str()),
            format!("operation '{}' should not be supported", op)
        );
    });

    let supported_ops: [&str; 4] = ["deleteOne", "deleteMany", "updateOne", "updateMany"];
    supported_ops.iter().for_each(|op| {
        assert!(
            field_names.contains(&format!("{}Item", *op).as_str()),
            format!("operation '{}' should be supported", op)
        );
    });
}
#[test]
#[serial]
fn no_nested_create_upsert_exist_with_ignored_field_without_default_value() {
    let dm = r#"
    datasource pg {
        provider = "postgresql"
        url = "postgresql://"
    }

    model User {
      id       Int              @id
      /// This type is currently not supported.
      postId Int
      post Post @relation(fields: [postId], references: [id])
    }

    model Post {
        id Int @id
        title String
        unsupported String @ignore
        users User[]
    }
"#;
    let (query_schema, datamodel) = get_query_schema(dm);
    let dmmf = crate::dmmf::render_dmmf(&datamodel, Arc::new(query_schema));

    let post_nested_create_input = find_input_type(&dmmf, PRISMA_NAMESPACE, "UserCreateInput");
    iterate_input_type_fields(&post_nested_create_input, &dmmf, &|_, field, parent_type| {
        if parent_type.name.contains("Post") {
            assert_ne!(
                field.name, "create",
                "nested operation '{}.{}' should not be available",
                parent_type.name, field.name
            );
            assert_ne!(
                field.name, "connectOrCreate",
                "nested operation '{}.{}' should not be available",
                parent_type.name, field.name
            );
        }
    });

    let post_nested_update_input = find_input_type(&dmmf, PRISMA_NAMESPACE, "UserUpdateInput");
    iterate_input_type_fields(&post_nested_update_input, &dmmf, &|_, field, parent_type| {
        if parent_type.name.contains("Post") {
            assert_ne!(
                field.name, "create",
                "nested operation '{}.{}' should not be available",
                parent_type.name, field.name
            );
            assert_ne!(
                field.name, "createMany",
                "nested operation '{}.{}' should not be available",
                parent_type.name, field.name
            );
            assert_ne!(
                field.name, "upsert",
                "nested operation '{}.{}' should not be available",
                parent_type.name, field.name
            );
            assert_ne!(
                field.name, "connectOrCreate",
                "nested operation '{}.{}' should not be available",
                parent_type.name, field.name
            );
        }
    });
}

#[test]
#[serial]
fn all_write_ops_should_exist_with_ignored_field_with_default_value() {
    let dm = r#"
      datasource pg {
          provider = "postgresql"
          url = "postgresql://"
      }

      model Item {
        id       Int              @id
        /// This type is currently not supported.
        index_a  String @ignore @default(dbgenerated("X"))
      }
  "#;
    let (query_schema, datamodel) = get_query_schema(dm);
    let dmmf = crate::dmmf::render_dmmf(&datamodel, Arc::new(query_schema));
    let mutation_type = dmmf
        .schema
        .output_object_types
        .get(PRISMA_NAMESPACE)
        .unwrap()
        .iter()
        .find(|o| o.name == "Mutation")
        .unwrap();

    let field_names: Vec<&str> = mutation_type.fields.iter().map(|f| f.name.as_str()).collect();

    let supported_ops: [&str; 6] = [
        "createOne",
        "upsertOne",
        "deleteOne",
        "deleteMany",
        "updateOne",
        "updateMany",
    ];

    supported_ops.iter().for_each(|op| {
        assert!(field_names.contains(&format!("{}Item", *op).as_str()));
    });
}

#[test]
#[serial]
fn ignored_models_should_be_filtered() {
    let dm = r#"
      datasource pg {
          provider = "postgresql"
          url = "postgresql://"
      }

      model Item {
        id       Int              @id
        a  String

        @@ignore
      }
  "#;
    let (query_schema, datamodel) = get_query_schema(dm);
    let dmmf = crate::dmmf::render_dmmf(&datamodel, Arc::new(query_schema));

    let query = find_output_type(&dmmf, PRISMA_NAMESPACE, "Query");
    let mutation = find_output_type(&dmmf, PRISMA_NAMESPACE, "Mutation");
    let has_no_inputs = dmmf.schema.input_object_types.get(PRISMA_NAMESPACE).is_none();

    assert_eq!(has_no_inputs, true);
    assert_eq!(query.fields.len(), 0);
    assert_eq!(mutation.fields.len(), 0);
}
