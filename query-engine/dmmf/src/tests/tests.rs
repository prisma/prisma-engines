use crate::{dmmf_from_schema, tests::setup::*};
use serde_json;

#[test]
fn sqlite_ignore() {
    let dmmf = dmmf_from_schema(include_str!("./test-schemas/sqlite_ignore.prisma"));

    // The Post model is ignored.
    assert_eq!(dmmf.data_model.models.len(), 1);
    assert_eq!(dmmf.mappings.model_operations.len(), 1);
}

const SNAPSHOTS_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src",
    "/tests",
    "/test-schemas",
    "/snapshots",
);

#[test]
/// This tests generate
fn dmmf_rendering() {
    let dmmf = dmmf_from_schema(include_str!("./test-schemas/odoo.prisma"));
    let snapshot_path = format!("{SNAPSHOTS_PATH}/odoo.snapshot.json.gz");

    if std::env::var("UPDATE_EXPECT").as_deref() == Ok("1") {
        write_compresed_snapshot(&dmmf, &snapshot_path);
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
