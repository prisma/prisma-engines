use crate::{dmmf_from_schema, tests::setup::*};

#[test]
fn sqlite_ignore() {
    let dmmf = dmmf_from_schema(include_str!("./test-schemas/sqlite_ignore.prisma"));

    // The Post model is ignored.
    assert_eq!(dmmf.data_model.models.len(), 1);
    assert_eq!(dmmf.mappings.model_operations.len(), 1);
}

#[test]
fn mongo_docs() {
    let dmmf = dmmf_from_schema(include_str!("./test-schemas/mongo.prisma"));

    for it in dmmf.data_model.types.iter() {
        assert_eq!(it.name, "Post");
        assert!(it.documentation.as_ref().is_some_and(|x| x.as_str() == "Post comment"));

        let mut fields = it.fields.iter();
        assert!(fields.any(|f| f.name == "published"
            && f.documentation
                .as_ref()
                .is_some_and(|x| x.as_str() == "published comment")));
        assert!(fields.any(|f| f.name == "authorId"
            && f.documentation
                .as_ref()
                .is_some_and(|x| x.as_str() == "authorId comment")));
    }
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
