use crate::{dmmf_from_schema, tests::setup::*};

#[test]
fn geometry_fields_in_datamodel_and_schema_dmmf() {
    let schema = r#"
        datasource db {
            provider = "postgresql"
        }

        generator client {
            provider = "prisma-client"
        }

        model Location {
            id       Int                      @id
            position Geometry(Point, 4326)
            path     Geometry(LineString)?
        }
    "#;

    let dmmf = dmmf_from_schema(schema);
    let location = dmmf
        .data_model
        .models
        .iter()
        .find(|m| m.name == "Location")
        .expect("Location model");
    let pos = location.fields.iter().find(|f| f.name == "position").unwrap();
    assert_eq!(pos.field_type, "geometry(Point,4326)");
    let path = location.fields.iter().find(|f| f.name == "path").unwrap();
    assert_eq!(path.field_type, "geometry(LineString,0)");

    let schema_json = serde_json::to_value(&dmmf.schema).unwrap();
    let models = schema_json
        .get("outputObjectTypes")
        .and_then(|v| v.get("model"))
        .and_then(|v| v.as_array())
        .expect("model output types");
    let location_out = models
        .iter()
        .find(|m| m.get("name").and_then(|n| n.as_str()) == Some("Location"))
        .expect("Location output type");
    let fields = location_out.get("fields").and_then(|f| f.as_array()).unwrap();
    let pos_field = fields
        .iter()
        .find(|f| f.get("name").and_then(|n| n.as_str()) == Some("position"));
    let out_pos = pos_field.and_then(|f| f.get("outputType")).expect("position output");
    assert_eq!(
        out_pos.get("type").and_then(|t| t.as_str()),
        Some("geometry(Point,4326)")
    );
}

#[test]
fn sqlite_ignore() {
    let dmmf = dmmf_from_schema(include_str!("./test-schemas/sqlite_ignore.prisma"));

    // The Post model is ignored.
    assert_eq!(dmmf.data_model.models.len(), 1);
    assert_eq!(dmmf.mappings.model_operations.len(), 1);
}

#[test]
fn views_ignore() {
    let dmmf = dmmf_from_schema(include_str!("./test-schemas/views_ignore.prisma"));

    // The Ignored view is ignored.
    assert_eq!(dmmf.data_model.models.len(), 1);
    assert_eq!(dmmf.mappings.model_operations.len(), 1);
}

fn assert_comment(actual: Option<&String>, expected: &str) {
    match actual {
        Some(actual) => assert_eq!(actual.as_str(), expected),
        None => panic!("Expected comment: {expected}"),
    }
}

#[test]
fn mongo_docs() {
    let dmmf = dmmf_from_schema(include_str!("./test-schemas/mongo.prisma"));

    for it in dmmf.data_model.types.iter() {
        assert_eq!(it.name, "Post");
        assert_comment(it.documentation.as_ref(), "Post comment");
        for field in it.fields.iter() {
            let name = field.name.as_str();
            let comment = field.documentation.as_ref();
            match name {
                "published" => assert_comment(comment, "published comment"),
                "authorId" => assert_comment(comment, "authorId comment"),
                _ => assert!(comment.as_ref().is_none()),
            };
        }
    }
}

#[test]
fn enum_docs() {
    let dmmf = dmmf_from_schema(include_str!("./test-schemas/postgres.prisma"));

    for it in dmmf.data_model.enums.iter() {
        assert_eq!(it.name, "Role");
        assert_comment(it.documentation.as_ref(), "Role enum comment");

        for field in it.values.iter() {
            let name = field.name.as_str();
            let comment = field.documentation.as_ref();
            match name {
                "USER" => assert_comment(comment, "user comment"),
                "ADMIN" => assert_comment(comment, "admin comment"),
                _ => assert!(comment.as_ref().is_none()),
            };
        }
    }
}

// Regression test for https://github.com/prisma/prisma/issues/19694
#[test]
fn unsupported_in_composite_type() {
    let schema = r#"
        generator client {
            provider = "prisma-client"
        }

        datasource db {
            provider = "mongodb"
        }

        type NestedType {
            this_causes_error Unsupported("RegularExpression")
        }

        model sample_model {
            id         String     @id @default(auto()) @map("_id") @db.ObjectId
            some_field NestedType
        }
    "#;

    dmmf_from_schema(schema);
}

// Regression test for https://github.com/prisma/prisma/issues/20986
#[test]
fn unusupported_in_compound_unique_must_not_panic() {
    let schema = r#"
        datasource db {
            provider = "postgresql"
        }

        generator client {
            provider = "postgresql"
        }

        model A {
            id          Int                      @id
            field       Int
            unsupported Unsupported("tstzrange")

            @@unique([field, unsupported])
        }
    "#;

    dmmf_from_schema(schema);
}

#[test]
fn unusupported_in_compound_id_must_not_panic() {
    let schema = r#"
        datasource db {
            provider = "postgresql"
        }

        generator client {
            provider = "postgresql"
        }

        model A {
            field       Int                      @unique
            unsupported Unsupported("tstzrange")

            @@id([field, unsupported])
        }
    "#;

    dmmf_from_schema(schema);
}

const SNAPSHOTS_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src",
    "/tests",
    "/test-schemas",
    "/snapshots",
);

#[test]
/// This test compares the dmmf output of odoo.prisma against a gzipped snapshot.
/// If you need to update the snapshot, add `UPDATE_EXPECT=1` to your environment variables.
fn dmmf_rendering() {
    let dmmf = dmmf_from_schema(include_str!("../../../schema/test-schemas/odoo.prisma"));
    let snapshot_path = format!("{SNAPSHOTS_PATH}/odoo.snapshot.json.gz");

    if std::env::var("UPDATE_EXPECT").as_deref() == Ok("1") {
        write_compressed_snapshot(&dmmf, &snapshot_path);
        return;
    }

    let new_dmmf = serde_json::to_value(&dmmf).unwrap();
    let old_dmmf = read_compressed_snapshot(&snapshot_path);

    if old_dmmf == new_dmmf {
        return; // test passed
    }

    panic_with_diff(
        &serde_json::to_string_pretty(&old_dmmf).unwrap(),
        &serde_json::to_string_pretty(&new_dmmf).unwrap(),
    );
}
