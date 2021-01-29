use std::sync::Arc;

use super::helpers::*;
use query_core::PRISMA_NAMESPACE;
use serial_test::serial;

// Tests in this file run serially because the function `get_query_schema` depends on setting an env var.

// Read

#[test]
#[serial]
fn unsupported_fields_should_be_filtered_from_input_and_output_types() {
    let dm = r#"
        datasource pg {
            provider = "postgresql"
            url = "postgresql://"
        }

        model Item {
            id           String @id
            unsupported_field Unsupported("X")
        }
    "#;
    let (query_schema, datamodel) = get_query_schema(dm);
    let dmmf = crate::dmmf::render_dmmf(&datamodel, Arc::new(query_schema));
    dbg!(&dmmf);
    let output_types = dmmf.schema.output_object_types.get(PRISMA_NAMESPACE).unwrap();
    let input_types = dmmf.schema.input_object_types.get(PRISMA_NAMESPACE).unwrap();

    for o in output_types {
        iterate_output_type_fields(o, &dmmf, &|field, _| assert_ne!(field.name, "unsupported_field"))
    }

    for o in input_types {
        iterate_input_type_fields(o, &dmmf, &|_, field, _| assert_ne!(field.name, "unsupported_field"))
    }
}

#[test]
#[serial]
fn unsupported_compound_indices_should_be_filtered_from_input_and_output_types() {
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
          a Unsupported("X")
          b Unsupported("X")
          c Unsupported("X")
          d Unsupported("X")

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
fn relation_with_unsupported_fk_fields_should_be_filtered_from_input_output_types() {
    let dm = r#"
        datasource pg {
            provider = "postgresql"
            url = "postgresql://"
        }
        model Post {
          id                Int                              @id @default(autoincrement())
          /// This type is currently not supported.
          unsupported_field  Unsupported("X") @default(dbgenerated("X"))
          user              User                             @relation(fields: [unsupported_field], references: [balance])
        }

        model User {
          id            Int                               @id @default(autoincrement())
          /// This type is currently not supported.
          balance       Unsupported("X")  @unique
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
fn no_find_unique_when_model_only_has_unsupported_index_or_compound() {
    let dm = r#"
        datasource pg {
            provider = "postgresql"
            url = "postgresql://"
        }

        model ItemA {
          id                Int
          /// This type is currently not supported.
          unsupported_index_a  Unsupported("X")  @id
          unsupported_index_c  Unsupported("X")  @unique
          unsupported_index_d  Unsupported("X")  @unique @default(dbgenerated("X"))
        }

        model ItemB {
          id                Int
          /// This type is currently not supported.
          unsupported_index_a  Unsupported("X")  @id @default(dbgenerated("X"))
        }

        model ItemC {
          id Int
          unsupported_index_a Unsupported("X")
          unsupported_index_b Unsupported("X")
          unsupported_index_c Unsupported("X")
          unsupported_index_d Unsupported("X")

          @@index([unsupported_index_a, unsupported_index_b])
          @@unique([unsupported_index_c, unsupported_index_d])
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

    assert!(field_names.contains(&"findUniqueItemA") == false);
    assert!(field_names.contains(&"findUniqueItemB") == false);
    assert!(field_names.contains(&"findUniqueItemC") == false);
}

// Write

#[test]
#[serial]
fn no_create_or_upsert_should_exist_with_unsupported_field_without_default_value() {
    let dm = r#"
        datasource pg {
            provider = "postgresql"
            url = "postgresql://"
        }

        model Item {
          id       Int              @id
          /// This type is currently not supported.
          index_a  Unsupported("X")
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

    let unsupported_ops: [&str; 3] = ["createOne", "createMany", "upsertOne"];
    unsupported_ops.iter().for_each(|op| {
        assert!(
            field_names.contains(&format!("{}Item", *op).as_str()) == false,
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
fn all_write_ops_should_exist_with_unsupported_field_with_default_value() {
    let dm = r#"
      datasource pg {
          provider = "postgresql"
          url = "postgresql://"
      }

      model Item {
        id       Int              @id
        /// This type is currently not supported.
        index_a  Unsupported("X") @default(dbgenerated("X"))
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
