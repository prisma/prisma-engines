use crate::{dmmf_from_schema, tests::setup::*};

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
    assert!(actual.is_some_and(|c| c.as_str() == expected))
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
            provider = "prisma-client-js"
        }

        datasource db {
            provider = "mongodb"
            url      = env("DATABASE_URL")
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
